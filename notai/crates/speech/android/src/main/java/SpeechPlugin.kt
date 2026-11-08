// NotAI konuşma tanıma (2026-11-08). Telefonun tanıyıcısı anında metin verir; Android 13+'ta
// aynı ses bizim kaydımızdan tanıyıcıya akıtılır ve WAV olarak saklanır (ev sunucusundaki
// Whisper metni sonra düzeltir). Eski Android'de yalnız metin, tanıyıcı yoksa yalnız ses.

package app.notai.speech

import android.Manifest
import android.annotation.SuppressLint
import android.app.Activity
import android.content.Intent
import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaRecorder
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.ParcelFileDescriptor
import android.speech.RecognitionListener
import android.speech.RecognizerIntent
import android.speech.SpeechRecognizer
import app.tauri.PermissionState
import app.tauri.annotation.Command
import app.tauri.annotation.Permission
import app.tauri.annotation.PermissionCallback
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.FileOutputStream
import java.io.OutputStream
import java.nio.ByteBuffer
import java.nio.ByteOrder

private const val MIC = "microphone"
private const val RATE = 16000
// Ek sınırı 10 MB: 16 kHz, 16 bit, mono = 32 KB/sn → en çok 280 sn ses saklanır (metin sürer).
private const val MAX_AUDIO_BYTES = RATE * 2 * 280

@TauriPlugin(
  permissions = [
    Permission(strings = [Manifest.permission.RECORD_AUDIO], alias = MIC)
  ]
)
class SpeechPlugin(private val activity: Activity) : Plugin(activity) {
  private val main = Handler(Looper.getMainLooper())
  private var recognizer: SpeechRecognizer? = null
  private var record: AudioRecord? = null
  private var reader: Thread? = null
  @Volatile private var running = false
  @Volatile private var pipeOut: OutputStream? = null
  private var pipeIn: ParcelFileDescriptor? = null
  private val pcm = ByteArrayOutputStream()
  private val finals = StringBuilder()
  private var partial = ""
  private var listening = false
  private var recognizerEnded = false
  private var recognizerError: String? = null
  private var pendingStop: Invoke? = null
  private var withAudio = false
  private var segmented = false

  @Command
  fun start(invoke: Invoke) {
    if (getPermissionState(MIC) != PermissionState.GRANTED) {
      requestPermissionForAlias(MIC, invoke, "micCallback")
      return
    }
    begin(invoke)
  }

  @PermissionCallback
  private fun micCallback(invoke: Invoke) {
    if (getPermissionState(MIC) == PermissionState.GRANTED) {
      begin(invoke)
    } else {
      invoke.reject("Mikrofon izni verilmedi.")
    }
  }

  @Command
  fun stop(invoke: Invoke) {
    main.post {
      listening = false
      pendingStop = invoke
      stopRecording()
      val sr = recognizer
      if (sr == null) {
        recognizerEnded = true
      } else if (!segmented) {
        sr.stopListening()
      }
      maybeFinish()
      // Tanıyıcı cevap vermezse de takılmayalım.
      main.postDelayed({ finish() }, 5000)
    }
  }

  @Command
  fun cancel(invoke: Invoke) {
    main.post {
      listening = false
      pendingStop?.reject("İptal edildi.")
      pendingStop = null
      cleanup()
      invoke.resolve()
    }
  }

  private fun begin(invoke: Invoke) {
    main.post {
      try {
        cleanup()
        reset()
        val available = SpeechRecognizer.isRecognitionAvailable(activity)
        withAudio = Build.VERSION.SDK_INT >= 33 || !available
        segmented = Build.VERSION.SDK_INT >= 33 && available
        listening = true
        if (withAudio) startRecording(segmented)
        if (available) startRecognizer() else recognizerEnded = true
        val r = JSObject()
        r.put("recognizer", available)
        r.put("audio", withAudio)
        invoke.resolve(r)
      } catch (e: Exception) {
        listening = false
        cleanup()
        invoke.reject("Mikrofon açılamadı: ${e.message}")
      }
    }
  }

  @SuppressLint("MissingPermission")
  private fun startRecording(feedRecognizer: Boolean) {
    val min = AudioRecord.getMinBufferSize(
      RATE, AudioFormat.CHANNEL_IN_MONO, AudioFormat.ENCODING_PCM_16BIT
    )
    val rec = AudioRecord(
      MediaRecorder.AudioSource.VOICE_RECOGNITION,
      RATE,
      AudioFormat.CHANNEL_IN_MONO,
      AudioFormat.ENCODING_PCM_16BIT,
      maxOf(min, RATE)
    )
    if (rec.state != AudioRecord.STATE_INITIALIZED) {
      rec.release()
      throw IllegalStateException("ses kaydı başlatılamadı")
    }
    record = rec
    if (feedRecognizer) {
      val pipe = ParcelFileDescriptor.createPipe()
      pipeIn = pipe[0]
      pipeOut = ParcelFileDescriptor.AutoCloseOutputStream(pipe[1])
    }
    running = true
    rec.startRecording()
    reader = Thread {
      val buf = ByteArray(3200)
      while (running) {
        val n = rec.read(buf, 0, buf.size)
        if (n > 0) {
          synchronized(pcm) {
            if (pcm.size() + n <= MAX_AUDIO_BYTES) pcm.write(buf, 0, n)
          }
          try {
            pipeOut?.write(buf, 0, n)
          } catch (e: Exception) {
            pipeOut = null
          }
        }
      }
      // Akışın sonu: bölümlü oturum biter, son metin gelir.
      try {
        pipeOut?.close()
      } catch (e: Exception) {
      }
      pipeOut = null
    }.also { it.start() }
  }

  private fun stopRecording() {
    running = false
    try {
      record?.stop()
    } catch (e: Exception) {
    }
    try {
      reader?.join(800)
    } catch (e: InterruptedException) {
    }
    reader = null
    record?.release()
    record = null
  }

  private fun startRecognizer() {
    val sr = SpeechRecognizer.createSpeechRecognizer(activity)
    recognizer = sr
    sr.setRecognitionListener(listener)
    sr.startListening(intent())
  }

  private fun intent(): Intent {
    val i = Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH)
    i.putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM)
    i.putExtra(RecognizerIntent.EXTRA_LANGUAGE, "tr-TR")
    i.putExtra(RecognizerIntent.EXTRA_PARTIAL_RESULTS, true)
    i.putExtra(RecognizerIntent.EXTRA_MAX_RESULTS, 1)
    val src = pipeIn
    if (segmented && src != null && Build.VERSION.SDK_INT >= 33) {
      i.putExtra(RecognizerIntent.EXTRA_AUDIO_SOURCE, src)
      i.putExtra(RecognizerIntent.EXTRA_AUDIO_SOURCE_CHANNEL_COUNT, 1)
      i.putExtra(RecognizerIntent.EXTRA_AUDIO_SOURCE_ENCODING, AudioFormat.ENCODING_PCM_16BIT)
      i.putExtra(RecognizerIntent.EXTRA_AUDIO_SOURCE_SAMPLING_RATE, RATE)
      i.putExtra(RecognizerIntent.EXTRA_SEGMENTED_SESSION, RecognizerIntent.EXTRA_AUDIO_SOURCE)
    } else {
      // Eski Android: uzun notta sessizlikte hemen bitirmesin (bitince yeniden başlatılır).
      i.putExtra(RecognizerIntent.EXTRA_SPEECH_INPUT_COMPLETE_SILENCE_LENGTH_MILLIS, 4000L)
      i.putExtra(RecognizerIntent.EXTRA_SPEECH_INPUT_POSSIBLY_COMPLETE_SILENCE_LENGTH_MILLIS, 4000L)
    }
    return i
  }

  private val listener = object : RecognitionListener {
    override fun onReadyForSpeech(params: Bundle?) {}
    override fun onBeginningOfSpeech() {}
    override fun onRmsChanged(rmsdB: Float) {}
    override fun onBufferReceived(buffer: ByteArray?) {}
    override fun onEndOfSpeech() {}
    override fun onEvent(eventType: Int, params: Bundle?) {}

    override fun onPartialResults(partialResults: Bundle?) {
      partial = first(partialResults)
      emitPartial()
    }

    override fun onResults(results: Bundle?) {
      addFinal(results)
      partial = ""
      emitPartial()
      if (listening && !segmented) {
        // Kullanıcı hâlâ konuşuyor: tanımayı sürdür.
        recognizer?.startListening(intent())
      } else {
        recognizerEnded = true
        maybeFinish()
      }
    }

    override fun onError(error: Int) {
      val quiet = error == SpeechRecognizer.ERROR_NO_MATCH ||
        error == SpeechRecognizer.ERROR_SPEECH_TIMEOUT
      if (quiet && listening && !segmented) {
        recognizer?.startListening(intent())
        return
      }
      if (!quiet) recognizerError = errorText(error)
      recognizerEnded = true
      maybeFinish()
    }

    override fun onSegmentResults(segmentResults: Bundle) {
      addFinal(segmentResults)
      partial = ""
      emitPartial()
    }

    override fun onEndOfSegmentedSession() {
      recognizerEnded = true
      maybeFinish()
    }
  }

  private fun first(b: Bundle?): String =
    b?.getStringArrayList(SpeechRecognizer.RESULTS_RECOGNITION)?.firstOrNull() ?: ""

  private fun addFinal(b: Bundle?) {
    val t = first(b).trim()
    if (t.isNotEmpty()) {
      if (finals.isNotEmpty()) finals.append(' ')
      finals.append(t)
    }
  }

  private fun currentText(): String = (finals.toString() + " " + partial).trim()

  private fun emitPartial() {
    val o = JSObject()
    o.put("text", currentText())
    trigger("partial", o)
  }

  private fun maybeFinish() {
    if (pendingStop != null && recognizerEnded) finish()
  }

  private fun finish() {
    val inv = pendingStop ?: return
    pendingStop = null
    val r = JSObject()
    r.put("text", currentText())
    val bytes = synchronized(pcm) { pcm.toByteArray() }
    // Yarım saniyeden kısa ses saklanmaz.
    if (withAudio && bytes.size > RATE) {
      try {
        val f = File(activity.cacheDir, "notai-voice-${System.currentTimeMillis()}.wav")
        writeWav(f, bytes)
        r.put("path", f.absolutePath)
      } catch (e: Exception) {
        r.put("error", "ses dosyası yazılamadı: ${e.message}")
      }
    }
    recognizerError?.let { if (!r.has("error")) r.put("error", it) }
    cleanup()
    inv.resolve(r)
  }

  private fun reset() {
    synchronized(pcm) { pcm.reset() }
    finals.setLength(0)
    partial = ""
    recognizerEnded = false
    recognizerError = null
  }

  private fun cleanup() {
    stopRecording()
    try {
      recognizer?.destroy()
    } catch (e: Exception) {
    }
    recognizer = null
    try {
      pipeOut?.close()
    } catch (e: Exception) {
    }
    pipeOut = null
    try {
      pipeIn?.close()
    } catch (e: Exception) {
    }
    pipeIn = null
  }

  private fun writeWav(f: File, data: ByteArray) {
    val header = ByteBuffer.allocate(44).order(ByteOrder.LITTLE_ENDIAN)
    header.put("RIFF".toByteArray(Charsets.US_ASCII))
    header.putInt(36 + data.size)
    header.put("WAVE".toByteArray(Charsets.US_ASCII))
    header.put("fmt ".toByteArray(Charsets.US_ASCII))
    header.putInt(16)
    header.putShort(1.toShort())
    header.putShort(1.toShort())
    header.putInt(RATE)
    header.putInt(RATE * 2)
    header.putShort(2.toShort())
    header.putShort(16.toShort())
    header.put("data".toByteArray(Charsets.US_ASCII))
    header.putInt(data.size)
    FileOutputStream(f).use {
      it.write(header.array())
      it.write(data)
    }
  }

  private fun errorText(code: Int): String = when (code) {
    SpeechRecognizer.ERROR_AUDIO -> "ses alınamadı"
    SpeechRecognizer.ERROR_NETWORK, SpeechRecognizer.ERROR_NETWORK_TIMEOUT ->
      "tanıma için internet gerekli; ses kaydedildi, ev sunucusu yazıya çevirecek"
    SpeechRecognizer.ERROR_INSUFFICIENT_PERMISSIONS -> "mikrofon izni yok"
    SpeechRecognizer.ERROR_RECOGNIZER_BUSY -> "tanıyıcı meşgul"
    SpeechRecognizer.ERROR_LANGUAGE_NOT_SUPPORTED, SpeechRecognizer.ERROR_LANGUAGE_UNAVAILABLE ->
      "Türkçe tanıma bu telefonda yok; ses ev sunucusunda yazıya çevrilecek"
    else -> "tanıma hatası ($code)"
  }
}

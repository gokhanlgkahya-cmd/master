//! Tarayıcı yapıştırıcısı (web-sys): tarih/saat, tema, dosya okuma, konum, ses kaydı.
//! Burada iş kuralı yoktur; yalnız tarayıcı API'lerine erişim ve Türkçe hata çevirisi.

use std::cell::RefCell;
use std::rc::Rc;

use chrono::{NaiveDate, NaiveTime};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use crate::view::{self, Theme};

const THEME_KEY: &str = "notai.theme";

/// Cihazın yerel tarihi.
#[must_use]
pub fn today() -> NaiveDate {
    let d = js_sys::Date::new_0();
    // getMonth 0 tabanlı; değerler her zaman aralıkta (Date nesnesi geçerli).
    NaiveDate::from_ymd_opt(
        i32::try_from(d.get_full_year()).unwrap_or(2026),
        d.get_month() + 1,
        d.get_date(),
    )
    .unwrap_or_default()
}

/// Cihazın yerel saati (dakika hassasiyetinde).
#[must_use]
pub fn now_time() -> NaiveTime {
    let d = js_sys::Date::new_0();
    NaiveTime::from_hms_opt(d.get_hours(), d.get_minutes(), 0).unwrap_or_default()
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

/// Kayıtlı tema seçimi (yoksa sistem).
#[must_use]
pub fn load_theme() -> Theme {
    storage()
        .and_then(|s| s.get_item(THEME_KEY).ok().flatten())
        .map_or(Theme::System, |v| Theme::parse(&v))
}

/// Temayı `<html data-theme>` özniteliğine uygular.
pub fn apply_theme(t: Theme) {
    let Some(root) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element())
    else {
        return;
    };
    let _ = match t.attr() {
        Some(v) => root.set_attribute("data-theme", v),
        None => root.remove_attribute("data-theme"),
    };
}

/// Temayı uygular ve seçimi saklar.
pub fn save_theme(t: Theme) {
    apply_theme(t);
    if let Some(s) = storage() {
        let _ = s.set_item(THEME_KEY, t.as_str());
    }
}

fn js_err(e: &JsValue) -> String {
    e.as_string()
        .or_else(|| {
            js_sys::Reflect::get(e, &JsValue::from_str("message"))
                .ok()
                .and_then(|m| m.as_string())
        })
        .unwrap_or_else(|| "bilinmeyen hata".to_owned())
}

fn js_name(e: &JsValue) -> String {
    js_sys::Reflect::get(e, &JsValue::from_str("name"))
        .ok()
        .and_then(|m| m.as_string())
        .unwrap_or_default()
}

async fn blob_bytes(blob: &web_sys::Blob) -> Result<Vec<u8>, String> {
    let buf = JsFuture::from(blob.array_buffer())
        .await
        .map_err(|e| format!("dosya okunamadı: {}", js_err(&e)))?;
    Ok(js_sys::Uint8Array::new(&buf).to_vec())
}

/// Seçilen dosya: ad, MIME türü ve baytlar.
pub struct Picked {
    pub name: String,
    pub mime: String,
    pub bytes: Vec<u8>,
}

/// `<input type=file>`'dan ilk dosyayı okur.
pub async fn read_first_file(input: &web_sys::HtmlInputElement) -> Result<Option<Picked>, String> {
    let Some(file) = input.files().and_then(|f| f.get(0)) else {
        return Ok(None);
    };
    let bytes = blob_bytes(&file).await?;
    let mime = file.type_();
    Ok(Some(Picked {
        name: file.name(),
        mime: if mime.is_empty() {
            "application/octet-stream".to_owned()
        } else {
            mime
        },
        bytes,
    }))
}

/// Seçilen görseli kare kırpıp `size` px JPEG'e küçültür (konu profil fotoğrafı). Sonuç
/// `data:image/jpeg;base64,…` (tipik 4–8 KB); konu kaydıyla senkronlanır.
pub async fn avatar_from_input(
    input: &web_sys::HtmlInputElement,
    size: u32,
) -> Result<Option<String>, String> {
    let Some(file) = input.files().and_then(|f| f.get(0)) else {
        return Ok(None);
    };
    let url = web_sys::Url::create_object_url_with_blob(&file).map_err(|e| js_err(&e))?;
    let img = web_sys::HtmlImageElement::new().map_err(|e| js_err(&e))?;
    img.set_src(&url);
    let decoded = JsFuture::from(img.decode()).await;
    let _ = web_sys::Url::revoke_object_url(&url);
    decoded.map_err(|_| "görsel açılamadı (desteklenmeyen biçim)".to_owned())?;
    let (w, h) = (
        f64::from(img.natural_width()),
        f64::from(img.natural_height()),
    );
    if w < 1.0 || h < 1.0 {
        return Err("görsel boş".into());
    }
    let side = w.min(h);
    let (sx, sy) = ((w - side) / 2.0, (h - side) / 2.0);
    let doc = web_sys::window()
        .and_then(|w| w.document())
        .ok_or("belge yok")?;
    let canvas: web_sys::HtmlCanvasElement = doc
        .create_element("canvas")
        .map_err(|e| js_err(&e))?
        .dyn_into()
        .map_err(|_| "tuval oluşturulamadı".to_owned())?;
    canvas.set_width(size);
    canvas.set_height(size);
    let ctx: web_sys::CanvasRenderingContext2d = canvas
        .get_context("2d")
        .map_err(|e| js_err(&e))?
        .ok_or("tuval bağlamı yok")?
        .dyn_into()
        .map_err(|_| "tuval bağlamı yok".to_owned())?;
    let d = f64::from(size);
    ctx.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
        &img, sx, sy, side, side, 0.0, 0.0, d, d,
    )
    .map_err(|e| js_err(&e))?;
    canvas
        .to_data_url_with_type_and_encoder_options("image/jpeg", &JsValue::from_f64(0.85))
        .map(Some)
        .map_err(|e| js_err(&e))
}

/// Cihaz konumu (enlem, boylam). İzin reddi Türkçe hata olarak döner.
pub async fn current_position() -> Result<(f64, f64), String> {
    let geo = web_sys::window()
        .ok_or("pencere yok")?
        .navigator()
        .geolocation()
        .map_err(|_| "Bu cihazda konum desteklenmiyor.".to_owned())?;
    let promise = js_sys::Promise::new(&mut |resolve, reject| {
        if let Err(e) = geo.get_current_position_with_error_callback(&resolve, Some(&reject)) {
            let _ = reject.call1(&JsValue::NULL, &e);
        }
    });
    let pos = JsFuture::from(promise).await.map_err(|e| {
        let code = js_sys::Reflect::get(&e, &JsValue::from_str("code"))
            .ok()
            .and_then(|c| c.as_f64())
            .map_or(0, |c| c as u32);
        view::geo_error(code)
    })?;
    let coords =
        js_sys::Reflect::get(&pos, &JsValue::from_str("coords")).map_err(|_| view::geo_error(0))?;
    let num = |k: &str| {
        js_sys::Reflect::get(&coords, &JsValue::from_str(k))
            .ok()
            .and_then(|v| v.as_f64())
            .ok_or_else(|| view::geo_error(0))
    };
    Ok((num("latitude")?, num("longitude")?))
}

/// Hareket teyidi için konum: (enlem, boylam, doğruluk m). En çok 8 sn bekler; 1 dk'lık
/// önbellekteki konum da kabul edilir (pil dostu).
pub async fn current_fix() -> Result<(f64, f64, f64), String> {
    let geo = web_sys::window()
        .ok_or("pencere yok")?
        .navigator()
        .geolocation()
        .map_err(|_| "Bu cihazda konum desteklenmiyor.".to_owned())?;
    let opts = web_sys::PositionOptions::new();
    opts.set_enable_high_accuracy(true);
    opts.set_timeout(8000);
    opts.set_maximum_age(60_000);
    let promise = js_sys::Promise::new(&mut |resolve, reject| {
        if let Err(e) =
            geo.get_current_position_with_error_callback_and_options(&resolve, Some(&reject), &opts)
        {
            let _ = reject.call1(&JsValue::NULL, &e);
        }
    });
    let pos = JsFuture::from(promise).await.map_err(|e| {
        let code = js_sys::Reflect::get(&e, &JsValue::from_str("code"))
            .ok()
            .and_then(|c| c.as_f64())
            .map_or(0, |c| c as u32);
        view::geo_error(code)
    })?;
    let coords =
        js_sys::Reflect::get(&pos, &JsValue::from_str("coords")).map_err(|_| view::geo_error(0))?;
    let num = |k: &str| {
        js_sys::Reflect::get(&coords, &JsValue::from_str(k))
            .ok()
            .and_then(|v| v.as_f64())
            .ok_or_else(|| view::geo_error(0))
    };
    Ok((
        num("latitude")?,
        num("longitude")?,
        num("accuracy").unwrap_or(50.0),
    ))
}

/// Süren ses kaydı.
pub struct Recorder {
    rec: web_sys::MediaRecorder,
    stream: web_sys::MediaStream,
    chunks: Rc<RefCell<Vec<web_sys::Blob>>>,
    _on_data: Closure<dyn FnMut(web_sys::BlobEvent)>,
}

impl Recorder {
    /// Mikrofon iznini ister ve kaydı başlatır.
    pub async fn start() -> Result<Self, String> {
        let window = web_sys::window().ok_or("pencere yok")?;
        let nav = window.navigator();
        let supported = js_sys::Reflect::has(&nav, &JsValue::from_str("mediaDevices"))
            .unwrap_or(false)
            && js_sys::Reflect::has(&window, &JsValue::from_str("MediaRecorder")).unwrap_or(false);
        if !supported {
            return Err(view::mic_error("NotSupportedError"));
        }
        let devices = nav
            .media_devices()
            .map_err(|_| view::mic_error("NotSupportedError"))?;
        let c = web_sys::MediaStreamConstraints::new();
        c.set_audio(&JsValue::TRUE);
        let promise = devices
            .get_user_media_with_constraints(&c)
            .map_err(|e| view::mic_error(&js_name(&e)))?;
        let stream: web_sys::MediaStream = JsFuture::from(promise)
            .await
            .map_err(|e| view::mic_error(&js_name(&e)))?
            .dyn_into()
            .map_err(|_| view::mic_error(""))?;
        let rec = web_sys::MediaRecorder::new_with_media_stream(&stream)
            .map_err(|e| view::mic_error(&js_name(&e)))?;
        let chunks: Rc<RefCell<Vec<web_sys::Blob>>> = Rc::default();
        let sink = Rc::clone(&chunks);
        let on_data =
            Closure::<dyn FnMut(web_sys::BlobEvent)>::new(move |ev: web_sys::BlobEvent| {
                if let Some(b) = ev.data().filter(|b| b.size() > 0.0) {
                    sink.borrow_mut().push(b);
                }
            });
        rec.set_ondataavailable(Some(on_data.as_ref().unchecked_ref()));
        rec.start().map_err(|e| view::mic_error(&js_name(&e)))?;
        Ok(Self {
            rec,
            stream,
            chunks,
            _on_data: on_data,
        })
    }

    fn release(&self) {
        for t in self.stream.get_tracks().iter() {
            if let Ok(t) = t.dyn_into::<web_sys::MediaStreamTrack>() {
                t.stop();
            }
        }
    }

    /// Kaydı iptal eder (veri atılır).
    pub fn cancel(self) {
        self.rec.set_onstop(None);
        let _ = self.rec.stop();
        self.release();
    }

    /// Kaydı bitirir: (baytlar, MIME türü).
    pub async fn finish(self) -> Result<(Vec<u8>, String), String> {
        let rec = self.rec.clone();
        let stopped = js_sys::Promise::new(&mut |resolve, _| {
            rec.set_onstop(Some(&resolve));
        });
        if self.rec.stop().is_ok() {
            let _ = JsFuture::from(stopped).await;
        }
        self.release();
        let mime = {
            let m = self.rec.mime_type();
            if m.is_empty() {
                "audio/webm".to_owned()
            } else {
                m
            }
        };
        let parts = js_sys::Array::new();
        for b in self.chunks.borrow().iter() {
            parts.push(b);
        }
        let opts = web_sys::BlobPropertyBag::new();
        opts.set_type(&mime);
        let blob = web_sys::Blob::new_with_blob_sequence_and_options(&parts, &opts)
            .map_err(|e| format!("ses kaydı birleştirilemedi: {}", js_err(&e)))?;
        let bytes = blob_bytes(&blob).await?;
        if bytes.is_empty() {
            return Err("Ses kaydı boş; biraz daha uzun konuş.".to_owned());
        }
        // Sunucu MIME parametresini (codecs=…) beklemez.
        let base = mime
            .split(';')
            .next()
            .unwrap_or("audio/webm")
            .trim()
            .to_owned();
        Ok((bytes, base))
    }
}

/// Öğeye odaklanır (bir sonraki karede; öğe henüz çizilmemiş olabilir).
pub fn focus_later(el: leptos::prelude::NodeRef<leptos::html::Input>) {
    use leptos::prelude::GetUntracked;
    leptos::prelude::request_animation_frame(move || {
        if let Some(i) = el.get_untracked() {
            let _ = i.focus();
        }
    });
}

/// Tauri bildirim eklentisinin "actionPerformed" olayını (Android'de bildirime dokunma) dinler.
/// Tauri dışında ya da olayı desteklemeyen masaüstünde sessizce hiçbir şey yapmaz (orada
/// uygulama açılışındaki "son bildirim" şeridi kullanılır).
pub fn on_notification_tap(cb: impl Fn(serde_json::Value) + 'static) {
    plugin_listener("notification", "actionPerformed", cb);
}

/// `@tauri-apps/api`'nin `addPluginListener`'ının yaptığını yapar: geri çağrıyı kanal olarak
/// kaydeder (`plugin:<eklenti>|register_listener`; erişim listesi yılan_yazımı adı bekler, Tauri
/// yerel eklentiye camelCase iletir). Kanal mesajı `{ index, message }` sarmalından çıkarılır.
/// Tauri dışında ya da eklenti yoksa sessizce hiçbir şey yapmaz.
pub fn plugin_listener(plugin: &str, event: &str, cb: impl Fn(serde_json::Value) + 'static) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(internals) = js_sys::Reflect::get(&window, &JsValue::from_str("__TAURI_INTERNALS__"))
    else {
        return;
    };
    if internals.is_undefined() {
        return;
    }
    let get_fn = |name: &str| {
        js_sys::Reflect::get(&internals, &JsValue::from_str(name))
            .ok()
            .and_then(|f| f.dyn_into::<js_sys::Function>().ok())
    };
    let (Some(transform), Some(invoke)) = (get_fn("transformCallback"), get_fn("invoke")) else {
        return;
    };
    let handler = Closure::<dyn FnMut(JsValue)>::new(move |v: JsValue| {
        if let Ok(json) = serde_wasm_bindgen::from_value::<serde_json::Value>(v) {
            if json.get("end").is_some() {
                return;
            }
            cb(json.get("message").cloned().unwrap_or(json));
        }
    });
    let Ok(id) = transform.call2(&internals, handler.as_ref(), &JsValue::FALSE) else {
        return;
    };
    // Dinleyici uygulama boyunca yaşar.
    handler.forget();
    let args = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &args,
        &JsValue::from_str("event"),
        &JsValue::from_str(event),
    );
    let channel = format!("__CHANNEL__:{}", id.as_f64().unwrap_or_default());
    let _ = js_sys::Reflect::set(
        &args,
        &JsValue::from_str("handler"),
        &JsValue::from_str(&channel),
    );
    if let Ok(p) = invoke.call2(
        &internals,
        &JsValue::from_str(&format!("plugin:{plugin}|register_listener")),
        &args,
    ) && let Ok(p) = p.dyn_into::<js_sys::Promise>()
    {
        // Masaüstünde yerel eklenti yoktur; reddi yut (konsolda yakalanmamış hata olmasın).
        wasm_bindgen_futures::spawn_local(async move {
            let _ = JsFuture::from(p).await;
        });
    }
}

//! `notai` komut satırı aracı.
//!
//! S24'teki ajanların (Hermes, Claude Code — Termux/Ubuntu) NotAI ile çalışması için:
//! sınıflandırma işlerini al (`jobs`), cevapla (`answer`), kayıt yakala, gelen kutusunu,
//! görevleri, sabah planını gör, notlara soru sor. Bağlantı: `NOTAI_URL`, `NOTAI_TOKEN`.

#![forbid(unsafe_code)]

use std::time::Duration;

use notai_api::{AskView, JournalView, MorningPlanView, TasksView};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

pub const HELP: &str = "notai — NotAI ev sunucusu komut satırı aracı

Kullanım: notai <komut> [argümanlar]

Ajan (sınıflandırma) komutları:
  jobs [--limit N]           Sınıflandırılmayı bekleyen kayıtlar: her biri için sistem istemi,
                             kullanıcı mesajı ve cevabın uyması gereken JSON şeması (JSON).
  answer <kayıt-id> [dosya]  Cevap JSON'unu gönderir (dosya yoksa ya da '-' ise stdin'den).
                             Sunucu cevabı şemaya göre doğrular; uymayan öneriler atılır.
  fail <kayıt-id> <neden>    İşi yapamadığını bildirir (kayıt 'AI işleyemedi' olur).
  watches                    Kontrol zamanı gelmiş fiyat/ürün takipleri (ürün, bağlantılar, şema).
  watch-report <konu-id> [dosya]
                             Bulunan teklifleri gönderir; yalnız değişince konuya yazılır.
  assets                     Bugün fiyatı güncellenecek varlıklar (sembol, miktar, son değer).
  asset-price <konu-id> <birim-fiyat>
                             Birim fiyatı (TL) bildirir; değer = miktar × fiyat, ≥%2 değişim konuya yazılır.
  statements                 Kurallarla okunamayan banka ekstreleri (metin + şema, JSON).
  statement-answer <id> [dosya]
                             Ekstredeki hareketleri (şemaya uygun JSON) gönderir.

Günlük komutlar:
  capture <metin…>           Ana Günlük'e kayıt yakalar.
  inbox                      İşlenmemiş kayıtlar ve bekleyen öneriler.
  tasks                      Geciken / bugün / yaklaşan / tarihsiz görevler.
  plan                       Bugünün sabah planı.
  ask <soru…>                Notlara soru sorar (cevap + kaynak kayıtlar).

Ortam: NOTAI_URL (varsayılan http://127.0.0.1:8787), NOTAI_TOKEN (API belirteci).";

/// Ayrıştırılmış komut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cmd {
    Help,
    Jobs { limit: u32 },
    Answer { entry: String, file: Option<String> },
    Fail { entry: String, reason: String },
    Capture { text: String },
    Inbox,
    Tasks,
    Plan,
    Ask { question: String },
    Statements,
    Watches,
    Assets,
    AssetPrice { topic: String, unit_price: String },
    WatchReport { topic: String, file: Option<String> },
    StatementAnswer { id: String, file: Option<String> },
}

/// Argümanları (program adı hariç) ayrıştırır.
pub fn parse(args: &[String]) -> Result<Cmd, String> {
    let rest = |from: usize| args.get(from..).map(|a| a.join(" ")).unwrap_or_default();
    let need = |s: String, what: &str| {
        if s.trim().is_empty() {
            Err(format!("{what} gerekli. Yardım: notai help"))
        } else {
            Ok(s)
        }
    };
    match args.first().map(String::as_str) {
        None | Some("help" | "-h" | "--help") => Ok(Cmd::Help),
        Some("jobs") => {
            let limit = match (args.get(1).map(String::as_str), args.get(2)) {
                (Some("--limit"), Some(n)) => {
                    n.parse().map_err(|_| format!("geçersiz sayı: {n}"))?
                }
                (None, _) => 5,
                _ => return Err("kullanım: notai jobs [--limit N]".into()),
            };
            Ok(Cmd::Jobs { limit })
        }
        Some("answer") => Ok(Cmd::Answer {
            entry: need(args.get(1).cloned().unwrap_or_default(), "kayıt kimliği")?,
            file: args.get(2).filter(|f| *f != "-").cloned(),
        }),
        Some("statements") => Ok(Cmd::Statements),
        Some("watches") => Ok(Cmd::Watches),
        Some("assets") => Ok(Cmd::Assets),
        Some("asset-price") => Ok(Cmd::AssetPrice {
            topic: need(args.get(1).cloned().unwrap_or_default(), "konu kimliği")?,
            unit_price: need(args.get(2).cloned().unwrap_or_default(), "birim fiyat")?,
        }),
        Some("watch-report") => Ok(Cmd::WatchReport {
            topic: need(args.get(1).cloned().unwrap_or_default(), "konu kimliği")?,
            file: args.get(2).filter(|f| *f != "-").cloned(),
        }),
        Some("statement-answer") => Ok(Cmd::StatementAnswer {
            id: need(args.get(1).cloned().unwrap_or_default(), "ekstre kimliği")?,
            file: args.get(2).filter(|f| *f != "-").cloned(),
        }),
        Some("fail") => Ok(Cmd::Fail {
            entry: need(args.get(1).cloned().unwrap_or_default(), "kayıt kimliği")?,
            reason: need(rest(2), "neden")?,
        }),
        Some("capture") => Ok(Cmd::Capture {
            text: need(rest(1), "metin")?,
        }),
        Some("inbox") => Ok(Cmd::Inbox),
        Some("tasks") => Ok(Cmd::Tasks),
        Some("plan") => Ok(Cmd::Plan),
        Some("ask") => Ok(Cmd::Ask {
            question: need(rest(1), "soru")?,
        }),
        Some(other) => Err(format!("bilinmeyen komut: {other}. Yardım: notai help")),
    }
}

/// Sunucu istemcisi.
pub struct Client {
    http: reqwest::Client,
    base: String,
    token: Option<String>,
}

impl Client {
    pub fn new(base: impl Into<String>, token: Option<String>) -> Result<Self, String> {
        Ok(Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .map_err(|e| e.to_string())?,
            base: base.into().trim_end_matches('/').to_owned(),
            token: token.filter(|t| !t.is_empty()),
        })
    }

    fn auth(&self, b: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.token {
            Some(t) => b.bearer_auth(t),
            None => b,
        }
    }

    async fn read<R: DeserializeOwned>(resp: reqwest::Response) -> Result<R, String> {
        let status = resp.status();
        if status.is_success() {
            return resp.json().await.map_err(|e| e.to_string());
        }
        let body: Value = resp.json().await.unwrap_or(Value::Null);
        Err(body["error"].as_str().map_or_else(
            || format!("sunucu {status} döndü"),
            |e| format!("sunucu: {e}"),
        ))
    }

    async fn get<R: DeserializeOwned>(&self, path: &str) -> Result<R, String> {
        let r = self.auth(self.http.get(format!("{}/api{path}", self.base)));
        Self::read(r.send().await.map_err(|e| format!("bağlanılamadı: {e}"))?).await
    }

    async fn post<R: DeserializeOwned>(&self, path: &str, body: &Value) -> Result<R, String> {
        let r = self.auth(
            self.http
                .post(format!("{}/api{path}", self.base))
                .json(body),
        );
        Self::read(r.send().await.map_err(|e| format!("bağlanılamadı: {e}"))?).await
    }
}

fn pretty(v: &Value) -> String {
    serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string())
}

/// Komutu çalıştırır; ekrana yazılacak metni döner. `stdin`: `answer` için okunacak girdi.
pub async fn run(
    cmd: Cmd,
    c: &Client,
    stdin: impl FnOnce() -> Result<String, String>,
) -> Result<String, String> {
    match cmd {
        Cmd::Help => Ok(HELP.to_owned()),
        Cmd::Jobs { limit } => {
            let v: Value = c.get(&format!("/agent/jobs?limit={limit}")).await?;
            Ok(pretty(&v))
        }
        Cmd::Assets => {
            let v: Value = c.get("/agent/assets").await?;
            serde_json::to_string_pretty(&v).map_err(|e| e.to_string())
        }
        Cmd::AssetPrice { topic, unit_price } => {
            let v: Value = c
                .post(
                    &format!("/agent/assets/{topic}/price"),
                    &serde_json::json!({ "unit_price": unit_price }),
                )
                .await?;
            Ok(v["message"].as_str().unwrap_or("işlendi").to_owned())
        }
        Cmd::Watches => {
            let v: Value = c.get("/agent/watches").await?;
            serde_json::to_string_pretty(&v).map_err(|e| e.to_string())
        }
        Cmd::WatchReport { topic, file } => {
            let raw = match file {
                Some(f) => {
                    std::fs::read_to_string(&f).map_err(|e| format!("{f} okunamadı: {e}"))?
                }
                None => stdin()?,
            };
            let body: Value =
                serde_json::from_str(&raw).map_err(|e| format!("rapor geçerli JSON değil: {e}"))?;
            let v: Value = c
                .post(&format!("/agent/watches/{topic}/report"), &body)
                .await?;
            Ok(v["message"].as_str().unwrap_or("işlendi").to_owned())
        }
        Cmd::Statements => {
            let v: Value = c.get("/agent/statements").await?;
            serde_json::to_string_pretty(&v).map_err(|e| e.to_string())
        }
        Cmd::StatementAnswer { id, file } => {
            let raw = match file {
                Some(f) => {
                    std::fs::read_to_string(&f).map_err(|e| format!("{f} okunamadı: {e}"))?
                }
                None => stdin()?,
            };
            let body: Value =
                serde_json::from_str(&raw).map_err(|e| format!("cevap geçerli JSON değil: {e}"))?;
            let v: Value = c
                .post(&format!("/agent/statements/{id}/answer"), &body)
                .await?;
            Ok(v["message"].as_str().unwrap_or("içe aktarıldı").to_owned())
        }
        Cmd::Answer { entry, file } => {
            let raw = match file {
                Some(f) => {
                    std::fs::read_to_string(&f).map_err(|e| format!("{f} okunamadı: {e}"))?
                }
                None => stdin()?,
            };
            let body: Value =
                serde_json::from_str(&raw).map_err(|e| format!("cevap geçerli JSON değil: {e}"))?;
            let v: Value = c
                .post(&format!("/agent/jobs/{entry}/answer"), &body)
                .await?;
            Ok(format!(
                "{} öneri kaydedildi{}",
                v["saved"],
                match v["dropped"].as_array() {
                    Some(d) if !d.is_empty() => format!("; atılanlar: {}", pretty(&v["dropped"])),
                    _ => String::new(),
                }
            ))
        }
        Cmd::Fail { entry, reason } => {
            let v: Value = c
                .post(
                    &format!("/agent/jobs/{entry}/fail"),
                    &json!({ "error": reason }),
                )
                .await?;
            Ok(format!("kaydedildi (AI işleyemedi: {})", v["failed"]))
        }
        Cmd::Capture { text } => {
            let v: Value = c.post("/entries", &json!({ "text": text })).await?;
            Ok(format!("yakalandı: {}", v["id"].as_str().unwrap_or("?")))
        }
        Cmd::Inbox => {
            let j: JournalView = c.get("/journal?filter=unprocessed&limit=100").await?;
            let mut out = format!("{} işlenmemiş kayıt\n", j.unprocessed);
            for g in j.groups {
                out.push_str(&format!("\n— {} —\n", g.label));
                for e in g.items {
                    out.push_str(&format!("{}  {}  {}\n", e.time, e.id, e.text));
                    for s in e.suggestions.iter().filter(|s| s.is_pending()) {
                        out.push_str(&format!("        ✧ {} ({})\n", s.chip, s.id));
                    }
                }
            }
            Ok(out)
        }
        Cmd::Tasks => {
            let t: TasksView = c.get("/tasks").await?;
            let mut out = String::new();
            for (name, rows) in [
                ("Geciken", t.overdue),
                ("Bugün", t.today),
                ("Yaklaşan", t.upcoming),
                ("Tarihsiz", t.undated),
            ] {
                if rows.is_empty() {
                    continue;
                }
                out.push_str(&format!("{name}\n"));
                for r in rows {
                    let due = r
                        .due_date
                        .map(|d| d.format(" · %d.%m").to_string())
                        .unwrap_or_default();
                    out.push_str(&format!("  [ ] {}{due} · {}\n", r.title, r.priority_label));
                }
            }
            Ok(if out.is_empty() {
                "açık görev yok".into()
            } else {
                out
            })
        }
        Cmd::Plan => {
            let p: MorningPlanView = c.get("/morning-plan").await?;
            let mut out = format!("{}\n", p.headline);
            for i in p.items {
                out.push_str(&format!("  • {} — {}\n", i.title, i.reason));
            }
            if p.more > 0 {
                out.push_str(&format!("  +{} görev daha\n", p.more));
            }
            Ok(out)
        }
        Cmd::Ask { question } => {
            let a: AskView = c.post("/ask", &json!({ "question": question })).await?;
            let mut out = a
                .answer
                .unwrap_or_else(|| "(AI cevabı yok) İlgili kayıtlar:".into());
            out.push('\n');
            for s in a.sources {
                out.push_str(&format!("  - [{}] {}\n", s.when, s.text));
            }
            Ok(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(s: &str) -> Vec<String> {
        s.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn parses_commands() {
        assert_eq!(parse(&[]).unwrap(), Cmd::Help);
        assert_eq!(parse(&a("jobs")).unwrap(), Cmd::Jobs { limit: 5 });
        assert_eq!(parse(&a("jobs --limit 3")).unwrap(), Cmd::Jobs { limit: 3 });
        assert!(parse(&a("jobs --limit x")).is_err());
        assert_eq!(
            parse(&a("answer abc -")).unwrap(),
            Cmd::Answer {
                entry: "abc".into(),
                file: None
            }
        );
        assert_eq!(parse(&a("statements")).unwrap(), Cmd::Statements);
        assert_eq!(parse(&a("watches")).unwrap(), Cmd::Watches);
        assert_eq!(parse(&a("assets")).unwrap(), Cmd::Assets);
        assert_eq!(
            parse(&a("asset-price t1 3.400,50")).unwrap(),
            Cmd::AssetPrice {
                topic: "t1".into(),
                unit_price: "3.400,50".into()
            }
        );
        assert!(parse(&a("asset-price t1")).is_err());
        assert_eq!(
            parse(&a("watch-report t1 rapor.json")).unwrap(),
            Cmd::WatchReport {
                topic: "t1".into(),
                file: Some("rapor.json".into())
            }
        );
        assert_eq!(
            parse(&a("statement-answer s1 -")).unwrap(),
            Cmd::StatementAnswer {
                id: "s1".into(),
                file: None
            }
        );
        assert_eq!(
            parse(&a("capture Kostik tedarikçisini ara")).unwrap(),
            Cmd::Capture {
                text: "Kostik tedarikçisini ara".into()
            }
        );
        assert!(parse(&a("capture")).is_err());
        assert!(parse(&a("fail abc")).is_err());
        assert!(parse(&a("uç")).is_err());
    }
}

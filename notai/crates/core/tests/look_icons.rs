//! Otomatik seçilebilen her konu simgesi tasarımdaki sprite'ta (design/screens/*.html) var mı?
//! Yoksa arayüzde boş kutu görünürdü.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;

use notai_core::look::{auto_icons, kind_icon};
use notai_core::topic::ParaKind;

#[test]
fn every_topic_icon_exists_in_design_sprite() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../design/screens");
    let mut symbols = BTreeSet::new();
    for f in std::fs::read_dir(&dir).unwrap() {
        let p = f.unwrap().path();
        if p.extension().is_some_and(|x| x == "html") {
            let html = std::fs::read_to_string(&p).unwrap();
            for part in html.split("<symbol id=\"i-").skip(1) {
                symbols.insert(part.split('"').next().unwrap().to_owned());
            }
        }
    }
    let kinds = [
        ParaKind::Project { due: None },
        ParaKind::Area,
        ParaKind::Resource,
        ParaKind::Archive,
    ];
    let missing: Vec<&str> = auto_icons()
        .into_iter()
        .chain(kinds.iter().map(kind_icon))
        .filter(|i| !symbols.contains(*i))
        .collect();
    assert!(
        missing.is_empty(),
        "sprite'ta olmayan simgeler: {missing:?}"
    );
}

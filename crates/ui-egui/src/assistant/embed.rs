//! Save chat with file: the assistant's conversation about a document, written into its
//! `.figcraft` file when the person turns it on (off by default), so it travels with the file and
//! comes back when the file is opened on another computer.
//!
//! A `.figcraft` file is a ZIP archive: the conversation is one more entry,
//! `artcraft/assistant.json` (portable form, without this computer's conversation id, see
//! `Memory::to_json`), which readers skip like any entry they don't know. Saving with the option
//! off takes it out again. Exports (SVG, PDF, PNG…) are saved as they are.

use std::io::{Cursor, Read, Write};

use serde_json::Value;

/// The entry that holds the conversation.
pub const PART: &str = "artcraft/assistant.json";
/// The largest conversation entry read back.
const MAX_PART: u64 = 8 << 20;

/// Files the conversation can be saved in.
pub fn can_embed(path: &str) -> bool {
    std::path::Path::new(path).extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("figcraft"))
}

/// Has saves put the conversation `a` keeps for each file into it, and `a` read it back.
pub fn install(a: &mut super::Assistant) {
    let shelf = a.shelf.clone();
    figcraft_engine::assistant_chat::set_hook(Box::new(move |path, bytes| {
        if !can_embed(path) {
            return None;
        }
        let chat = shelf.lock().ok().and_then(|s| s.chat_for(path));
        embed(bytes, chat.as_ref())
    }));
    a.embedded_reader = Some(read);
}

/// The conversation saved in the file at `path`, if it has one.
pub fn read(path: &str) -> Option<Value> {
    if !can_embed(path) {
        return None;
    }
    read_from(std::fs::File::open(path).ok()?)
}

fn read_from(r: impl Read + std::io::Seek) -> Option<Value> {
    let mut zip = zip::ZipArchive::new(r).ok()?;
    let part = zip.by_name(PART).ok()?;
    let mut text = String::new();
    part.take(MAX_PART).read_to_string(&mut text).ok()?;
    serde_json::from_str(&text).ok()
}

/// The archive `bytes` with `chat` as its conversation entry (replacing one already there), or
/// without one when `chat` is None. None when nothing changes or `bytes` isn't a ZIP archive.
pub fn embed(bytes: &[u8], chat: Option<&Value>) -> Option<Vec<u8>> {
    let mut zin = zip::ZipArchive::new(Cursor::new(bytes)).ok()?;
    let has = zin.index_for_name(PART).is_some();
    match chat {
        None if !has => return None,
        Some(c) if has && read_from(Cursor::new(bytes)).as_ref() == Some(c) => return None,
        _ => {}
    }
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..zin.len() {
        if zin.name_for_index(i)? == PART {
            continue;
        }
        out.raw_copy_file(zin.by_index_raw(i).ok()?).ok()?;
    }
    if let Some(c) = chat {
        let deflate = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        out.start_file(PART, deflate).ok()?;
        out.write_all(c.to_string().as_bytes()).ok()?;
    }
    Some(out.finish().ok()?.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_chat_goes_into_the_file_and_the_document_still_opens() {
        let doc = figcraft_doc::Document::new("Trang chủ");
        let file = figcraft_format::save_bytes(&doc, None).unwrap();
        let chat = json!({"version": 1, "messages": [{"role": "user", "text": "Thêm nút Đăng nhập"}]});
        let with = embed(&file, Some(&chat)).unwrap();
        assert_eq!(read_from(Cursor::new(&with)), Some(chat.clone()));
        assert!(figcraft_format::load_bytes(&with).is_ok(), "the document opens with the conversation in it");
        assert_eq!(embed(&with, Some(&chat)), None, "the same conversation again: nothing to do");
        let without = embed(&with, None).unwrap();
        assert_eq!(read_from(Cursor::new(&without)), None, "with the option off it is taken out");
        assert_eq!(embed(&file, None), None, "nothing to do");
        assert_eq!(embed(b"not a zip", Some(&chat)), None);
        assert!(can_embed("C:/Thiết kế/Màn hình.FIGCRAFT") && !can_embed("a.fig"));
    }
}

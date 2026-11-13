use arboard::Clipboard;

#[derive(Default)]
pub(crate) struct ClipboardSlot {
    clipboard: Option<Clipboard>,
}

impl ClipboardSlot {
    pub(crate) fn copy(&mut self, text: String) -> Result<(), arboard::Error> {
        let clipboard = match &mut self.clipboard {
            Some(clipboard) => clipboard,
            empty_slot => empty_slot.insert(Clipboard::new()?),
        };
        clipboard.set_text(text)
    }
}

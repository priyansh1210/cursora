//! A small X11 client for capture and focus (spec 08), over x11rb's pure-Rust connection: nothing to install at run time.
use std::cell::Cell;
use std::time::{Duration, Instant};
use x11rb::connection::Connection;
use x11rb::protocol::xfixes::{ConnectionExt as _, SelectionEventMask};
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;
use x11rb::{CURRENT_TIME, NONE};

pub type R<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

x11rb::atom_manager! {
    pub Atoms: AtomsCookie {
        CLIPBOARD,
        UTF8_STRING,
        TARGETS,
        MULTIPLE,
        TIMESTAMP,
        SAVE_TARGETS,
        DELETE,
        INCR,
        _NET_ACTIVE_WINDOW,
        _NET_WM_PID,
        _NET_WM_NAME,
        URI_LIST: b"text/uri-list",
        GNOME_FILES: b"x-special/gnome-copied-files",
        CURSORA_SELECTION, // where owners put what we asked for
    }
}

/// One saved selection target, replayed exactly as the owner gave it.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub target: Atom,
    pub type_: Atom,
    pub format: u8,
    pub data: Vec<u8>,
}

/// One connection, used from one thread.
pub struct X11 {
    pub conn: RustConnection,
    pub root: Window,
    pub atoms: Atoms,
    /// Hidden; conversions land in its property, and it receives the clipboard owner-change events.
    window: Window,
    owner_changes: Cell<u32>,
}

impl X11 {
    pub fn connect() -> R<X11> {
        let (conn, screen) = x11rb::connect(None)?;
        let root = conn.setup().roots[screen].root;
        let atoms = Atoms::new(&conn)?.reply()?;
        let window = conn.generate_id()?;
        let aux = CreateWindowAux::new().event_mask(EventMask::PROPERTY_CHANGE);
        conn.create_window(0, window, root, 0, 0, 1, 1, 0, WindowClass::INPUT_ONLY, x11rb::COPY_FROM_PARENT, &aux)?;
        conn.xfixes_query_version(5, 0)?.reply()?;
        conn.xfixes_select_selection_input(window, atoms.CLIPBOARD, SelectionEventMask::SET_SELECTION_OWNER)?;
        conn.flush()?;
        Ok(X11 { conn, root, atoms, window, owner_changes: Cell::new(0) })
    }

    /// The whole property, or None when it isn't set.
    pub fn property(&self, window: Window, name: impl Into<Atom>, type_: impl Into<Atom>) -> R<Option<GetPropertyReply>> {
        let reply = self.conn.get_property(false, window, name, type_, 0, u32::MAX / 4)?.reply()?;
        Ok((reply.type_ != NONE).then_some(reply))
    }

    /// The first 32-bit value of a property: a window or a number.
    pub fn value32(&self, window: Window, name: impl Into<Atom>, type_: impl Into<Atom>) -> R<Option<u32>> {
        Ok(self.property(window, name, type_)?.and_then(|p| p.value32()?.next()))
    }

    pub fn active_window(&self) -> R<Window> {
        Ok(self.value32(self.root, self.atoms._NET_ACTIVE_WINDOW, AtomEnum::WINDOW)?.unwrap_or(NONE))
    }

    /// Asks the window manager to focus `window` as a pager would (source 2), which skips the
    /// user-timestamp check that blocks ordinary focus requests (issue 18).
    pub fn activate(&self, window: Window) -> R<()> {
        let event = ClientMessageEvent::new(32, window, self.atoms._NET_ACTIVE_WINDOW, [2, CURRENT_TIME, 0, 0, 0]);
        self.conn.send_event(false, self.root, EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY, event)?;
        self.conn.flush()?;
        Ok(())
    }

    /// Asks the owner of `selection` for `target`. Ok(None): no owner, or it refused. Err: no answer by `deadline`.
    pub fn convert(&self, selection: Atom, target: Atom, deadline: Instant) -> R<Option<Item>> {
        let property = self.atoms.CURSORA_SELECTION;
        self.conn.convert_selection(self.window, selection, target, property, CURRENT_TIME)?;
        self.conn.flush()?;
        let notify = loop {
            match self.next_event(deadline)? {
                Event::SelectionNotify(e) if e.requestor == self.window && e.selection == selection && e.target == target => break e,
                _ => {}
            }
        };
        if notify.property == NONE {
            return Ok(None);
        }
        let first = self.conn.get_property(true, self.window, property, AtomEnum::ANY, 0, u32::MAX / 4)?.reply()?;
        if first.type_ != self.atoms.INCR {
            return Ok(Some(Item { target, type_: first.type_, format: first.format, data: first.value }));
        }
        // INCR: deleting the property asked for the first chunk; an empty chunk ends the transfer.
        let mut item = Item { target, type_: NONE, format: 8, data: Vec::new() };
        loop {
            match self.next_event(deadline)? {
                Event::PropertyNotify(e) if e.window == self.window && e.atom == property && e.state == Property::NEW_VALUE => {
                    let chunk = self.conn.get_property(true, self.window, property, AtomEnum::ANY, 0, u32::MAX / 4)?.reply()?;
                    if chunk.value.is_empty() {
                        return Ok(Some(item));
                    }
                    (item.type_, item.format) = (chunk.type_, chunk.format);
                    item.data.extend(chunk.value);
                }
                _ => {}
            }
        }
    }

    /// How often the clipboard changed owner since this connection opened: the copy's sequence number (SEL-3.3).
    pub fn owner_changes(&self) -> u32 {
        while self.next_event(Instant::now()).is_ok() {}
        self.owner_changes.get()
    }

    /// The next event, or Err at the deadline. Clipboard owner changes are counted on the way.
    fn next_event(&self, deadline: Instant) -> R<Event> {
        loop {
            match self.conn.poll_for_event()? {
                Some(Event::XfixesSelectionNotify(_)) => self.owner_changes.set(self.owner_changes.get() + 1),
                Some(event) => return Ok(event),
                None if Instant::now() >= deadline => return Err("the X server or the selection owner didn't answer in time".into()),
                None => std::thread::sleep(Duration::from_millis(1)),
            }
        }
    }

    /// The keycode that types `keysym` without modifiers.
    pub fn keycode(&self, keysym: u32) -> R<Option<u8>> {
        let setup = self.conn.setup();
        let map = self.conn.get_keyboard_mapping(setup.min_keycode, setup.max_keycode - setup.min_keycode + 1)?.reply()?;
        let per = map.keysyms_per_keycode.max(1) as usize;
        Ok(map.keysyms.chunks(per).position(|syms| syms[0] == keysym).map(|i| setup.min_keycode + i as u8))
    }
}

/// Takes `selection` and hands out `items` exactly as saved, until another app takes it (the clipboard restore, SEL-3.1).
/// No items: the selection is left without an owner, as it was.
pub fn serve(selection: Atom, items: Vec<Item>) {
    std::thread::spawn(move || {
        let run = || -> R<()> {
            let x = X11::connect()?;
            let owner = if items.is_empty() { NONE } else { x.window };
            x.conn.set_selection_owner(owner, selection, CURRENT_TIME)?;
            // Asking who owns it also sends the request above, and checks that we got it (ICCCM).
            if x.conn.get_selection_owner(selection)?.reply()?.owner != owner || owner == NONE {
                return Ok(());
            }
            loop {
                match x.conn.wait_for_event()? {
                    Event::SelectionClear(e) if e.selection == selection => return Ok(()),
                    Event::SelectionRequest(r) => x.answer(&r, &items)?,
                    _ => {}
                }
            }
        };
        if let Err(e) = run() {
            log::warn!("clipboard restore: {e}");
        }
    });
}

impl X11 {
    fn answer(&self, r: &SelectionRequestEvent, items: &[Item]) -> R<()> {
        let property = if r.property == NONE { r.target } else { r.property }; // very old clients (ICCCM)
        let given = if r.target == self.atoms.TARGETS {
            let targets: Vec<Atom> = items.iter().map(|i| i.target).chain([self.atoms.TARGETS]).collect();
            self.conn.change_property32(PropMode::REPLACE, r.requestor, property, AtomEnum::ATOM, &targets)?;
            true
        } else if let Some(item) = items.iter().find(|i| i.target == r.target && i.format >= 8) {
            let len = item.data.len() as u32 / (item.format as u32 / 8);
            self.conn.change_property(PropMode::REPLACE, r.requestor, property, item.type_, item.format, len, &item.data)?;
            true
        } else {
            false
        };
        let notify = SelectionNotifyEvent {
            response_type: SELECTION_NOTIFY_EVENT,
            sequence: 0,
            time: r.time,
            requestor: r.requestor,
            selection: r.selection,
            target: r.target,
            property: if given { property } else { NONE },
        };
        self.conn.send_event(false, r.requestor, EventMask::NO_EVENT, notify)?;
        self.conn.flush()?;
        Ok(())
    }
}

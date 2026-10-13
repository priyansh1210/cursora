//! Privacy guard (spec 07): hiding personal details before a request leaves Cursora, and putting them back in the answer.
//! Local-only mode is `Settings.local_only` (spec 04) and the app blocklist is `CaptureSettings.blocklist`.
use crate::chat::{Locality, Message};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

/// Settings → Privacy.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct PrivacySettings {
    /// Hide personal details before a cloud request (PRI-3.1).
    pub redact: bool,
    /// Before a local or LAN request too (PRI-3.5).
    pub redact_local: bool,
    pub detectors_off: Vec<Detector>,
    pub patterns: Vec<Pattern>,
    /// Show "What will be sent" and wait for Send before a cloud request (PRI-3.7).
    pub confirm_cloud: bool,
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self { redact: true, redact_local: false, detectors_off: vec![], patterns: vec![], confirm_cloud: false }
    }
}

/// The user's own pattern (PRI-3.4). Its values become `[CUSTOM_<LABEL>_1]`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Pattern {
    pub label: String,
    pub regex: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Detector {
    ApiKey,
    Email,
    Iban,
    Card,
    Aadhaar,
    Pan,
    Phone,
}

struct Builtin {
    id: Detector,
    tag: &'static str,
    name: &'static str,
    pattern: &'static str,
    check: fn(&str) -> bool,
}

/// PRI-3.2, in priority order: where two matches overlap, the earlier detector wins.
/// Phone is last, so a card or Aadhaar number is never taken for a phone number.
const BUILTINS: [Builtin; 7] = [
    Builtin {
        id: Detector::ApiKey,
        tag: "APIKEY",
        name: "API keys and tokens",
        pattern: r"\b(?:sk-ant-[A-Za-z0-9_-]{20,}|sk-[A-Za-z0-9_-]{20,}|AKIA[0-9A-Z]{16}|ghp_[A-Za-z0-9]{36}|github_pat_[A-Za-z0-9_]{22,}|xox[bp]-[A-Za-z0-9-]{10,}|AIza[0-9A-Za-z_-]{35}|eyJ[A-Za-z0-9_-]+\.eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+)",
        check: any,
    },
    Builtin { id: Detector::Email, tag: "EMAIL", name: "Email addresses", pattern: r"(?i)\b[a-z0-9._%+-]+@[a-z0-9-]+(?:\.[a-z0-9-]+)*\.[a-z]{2,}\b", check: any },
    Builtin { id: Detector::Iban, tag: "IBAN", name: "IBANs", pattern: r"\b[A-Z]{2}[0-9]{2}(?:[A-Z0-9]{11,30}|(?: [A-Z0-9]{4}){2,7}(?: [A-Z0-9]{1,3})?)\b", check: mod97 },
    Builtin { id: Detector::Card, tag: "CARD", name: "Payment card numbers", pattern: r"\b[0-9](?:[ -]?[0-9]){12,18}\b", check: luhn },
    Builtin { id: Detector::Aadhaar, tag: "AADHAAR", name: "Aadhaar numbers", pattern: r"\b[2-9][0-9]{3} ?[0-9]{4} ?[0-9]{4}\b", check: verhoeff },
    Builtin { id: Detector::Pan, tag: "PAN", name: "PAN numbers", pattern: r"\b[A-Z]{5}[0-9]{4}[A-Z]\b", check: any },
    Builtin {
        id: Detector::Phone,
        tag: "PHONE",
        name: "Phone numbers",
        // E.164 with separators; Indian mobiles; (NNN) NNN-NNNN and NNN-NNN-NNNN.
        pattern: r"\+[1-9](?:[ .()-]{0,2}[0-9]){7,14}|\b(?:0|91[ -]?)?[6-9][0-9]{4}[ -]?[0-9]{5}\b|(?:\([0-9]{3}\) ?|\b[0-9]{3}[-.])[0-9]{3}[-.][0-9]{4}\b",
        check: any,
    },
];

static COMPILED: LazyLock<Vec<Regex>> = LazyLock::new(|| BUILTINS.iter().map(|b| Regex::new(b.pattern).expect("built-in pattern")).collect());

/// The longest placeholder a stream may hold back: `[CUSTOM_` + a 20-character label + `_` + the number + `]`.
const MAX_PLACEHOLDER: usize = 48;

fn any(_: &str) -> bool {
    true
}

fn digits(s: &str) -> Vec<u32> {
    s.chars().filter_map(|c| c.to_digit(10)).collect()
}

fn luhn(s: &str) -> bool {
    let sum: u32 = digits(s).iter().rev().enumerate().map(|(i, &d)| if i % 2 == 1 { if d * 2 > 9 { d * 2 - 9 } else { d * 2 } } else { d }).sum();
    sum % 10 == 0
}

/// The first four characters move to the end, letters become 10–35, and the number mod 97 must be 1.
fn mod97(s: &str) -> bool {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    let (head, rest) = s.split_at(4);
    rest.chars().chain(head.chars()).try_fold(0u32, |r, c| {
        let v = c.to_digit(36)?;
        Some(if v < 10 { (r * 10 + v) % 97 } else { (r * 100 + v) % 97 })
    }) == Some(1)
}

fn verhoeff(s: &str) -> bool {
    const D: [[u8; 10]; 10] = [
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        [1, 2, 3, 4, 0, 6, 7, 8, 9, 5],
        [2, 3, 4, 0, 1, 7, 8, 9, 5, 6],
        [3, 4, 0, 1, 2, 8, 9, 5, 6, 7],
        [4, 0, 1, 2, 3, 9, 5, 6, 7, 8],
        [5, 9, 8, 7, 6, 0, 4, 3, 2, 1],
        [6, 5, 9, 8, 7, 1, 0, 4, 3, 2],
        [7, 6, 5, 9, 8, 2, 1, 0, 4, 3],
        [8, 7, 6, 5, 9, 3, 2, 1, 0, 4],
        [9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
    ];
    const P: [[u8; 10]; 8] = [
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        [1, 5, 7, 6, 2, 8, 3, 0, 9, 4],
        [5, 8, 0, 3, 7, 9, 6, 1, 4, 2],
        [8, 9, 1, 6, 0, 4, 3, 5, 2, 7],
        [9, 4, 5, 3, 1, 2, 6, 8, 7, 0],
        [4, 2, 8, 6, 5, 7, 3, 9, 0, 1],
        [2, 7, 9, 3, 8, 0, 6, 4, 1, 5],
        [7, 0, 4, 6, 9, 1, 3, 2, 5, 8],
    ];
    digits(s).iter().rev().enumerate().fold(0, |c, (i, &d)| D[c as usize][P[i % 8][d as usize] as usize]) == 0
}

fn custom_tag(label: &str) -> String {
    format!("CUSTOM_{}", label.to_ascii_uppercase())
}

/// A pattern's regex, or why it can't be used (PRI-3.4).
fn compile(p: &Pattern) -> Result<Regex, String> {
    if p.label.is_empty() || p.label.len() > 20 || !p.label.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(format!("A pattern's label must be 1 to 20 letters or digits, not \"{}\".", p.label));
    }
    Regex::new(&p.regex).map_err(|e| format!("The pattern {} isn't a valid regular expression: {e}", p.label))
}

/// Checked when settings are saved (PRI-3.4).
pub fn validate(p: &PrivacySettings) -> Result<(), String> {
    p.patterns.iter().try_for_each(|pat| compile(pat).map(|_| ()))
}

/// Whether a request to a model of this locality is redacted (PRI-3.1, 3.5). LAN counts as local.
pub fn applies(p: &PrivacySettings, locality: Locality) -> bool {
    p.redact && (locality == Locality::Cloud || p.redact_local)
}

/// PRI-3.7: a cloud request waits for an explicit Send while "Confirm before sending to cloud" is on.
pub fn must_confirm(p: &PrivacySettings, locality: Locality, confirmed: bool) -> bool {
    p.confirm_cloud && locality == Locality::Cloud && !confirmed
}

/// PRI-3.8: redaction covers text only.
pub fn image_warning(messages: &[Message], locality: Locality) -> bool {
    locality == Locality::Cloud && messages.iter().any(|m| !m.images.is_empty())
}

/// The detectors switched on, the user's patterns first.
pub struct Detectors(Vec<(String, Regex, fn(&str) -> bool)>);

impl Detectors {
    pub fn new(p: &PrivacySettings) -> Result<Self, String> {
        let mut list = Vec::new();
        for pat in &p.patterns {
            list.push((custom_tag(&pat.label), compile(pat)?, any as fn(&str) -> bool));
        }
        for (b, re) in BUILTINS.iter().zip(COMPILED.iter()) {
            if !p.detectors_off.contains(&b.id) {
                list.push((b.tag.to_string(), re.clone(), b.check));
            }
        }
        Ok(Self(list))
    }

    /// `text` with every detected value replaced by its placeholder, numbered in `hidden`.
    pub fn hide(&self, text: &str, hidden: &mut Hidden) -> String {
        let mut found: Vec<(usize, usize, usize)> = Vec::new();
        for (i, (_, re, check)) in self.0.iter().enumerate() {
            for m in re.find_iter(text) {
                if !m.is_empty() && check(m.as_str()) && !found.iter().any(|&(s, e, _)| s < m.end() && m.start() < e) {
                    found.push((m.start(), m.end(), i));
                }
            }
        }
        found.sort_unstable();
        let mut out = String::with_capacity(text.len());
        let mut last = 0;
        for (start, end, i) in found {
            out.push_str(&text[last..start]);
            out.push_str(&hidden.placeholder(&self.0[i].0, &text[start..end]));
            last = end;
        }
        out + &text[last..]
    }
}

/// The values hidden from one request, as (placeholder, value) in the order they were found (PRI-3.1).
#[derive(Default, Debug, Clone, PartialEq)]
pub struct Hidden(Vec<(String, String)>);

impl Hidden {
    /// The value's placeholder: the one it already has, else the next number for its type.
    fn placeholder(&mut self, tag: &str, value: &str) -> String {
        if let Some((p, _)) = self.0.iter().find(|(_, v)| v == value) {
            return p.clone();
        }
        let prefix = format!("[{tag}_");
        let p = format!("{prefix}{}]", self.0.iter().filter(|(p, _)| p.starts_with(&prefix)).count() + 1);
        self.0.push((p.clone(), value.to_string()));
        p
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn placeholders(&self) -> Vec<String> {
        self.0.iter().map(|(p, _)| p.clone()).collect()
    }

    /// Puts the values back (PRI-3.3). A placeholder the model made up stays as written.
    pub fn restore(&self, text: &str) -> String {
        self.0.iter().fold(text.to_string(), |t, (p, v)| t.replace(p.as_str(), v))
    }
}

/// Restores a streamed answer piece by piece; a placeholder split across pieces is held back until it is whole (PRI-3.3).
pub struct Restorer {
    hidden: Hidden,
    held: String,
}

impl Restorer {
    pub fn new(hidden: Hidden) -> Self {
        Self { hidden, held: String::new() }
    }

    /// What can be shown now.
    pub fn push(&mut self, piece: &str) -> String {
        if self.hidden.is_empty() {
            return piece.to_string();
        }
        self.held.push_str(piece);
        let partial = |i: usize| self.held.len() - i <= MAX_PLACEHOLDER && self.held[i + 1..].chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
        let keep = self.held.rfind('[').filter(|&i| partial(i)).unwrap_or(self.held.len());
        let tail = self.held.split_off(keep);
        let out = self.hidden.restore(&self.held);
        self.held = tail;
        out
    }

    /// The rest, when the stream ends, stops or fails.
    pub fn finish(&mut self) -> String {
        let out = self.hidden.restore(&self.held);
        self.held.clear();
        out
    }
}

#[derive(Serialize)]
pub struct DetectorInfo {
    id: Detector,
    name: &'static str,
}

/// The built-in detectors for Settings → Privacy, so their names are written once.
#[tauri::command]
pub fn privacy_detectors() -> Vec<DetectorInfo> {
    BUILTINS.iter().map(|b| DetectorInfo { id: b.id, name: b.name }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::{Image, Role};

    fn hide(p: &PrivacySettings, text: &str) -> (String, Hidden) {
        let mut hidden = Hidden::default();
        (Detectors::new(p).unwrap().hide(text, &mut hidden), hidden)
    }

    fn found(text: &str) -> Vec<String> {
        hide(&PrivacySettings::default(), text).1 .0.into_iter().map(|(p, v)| format!("{p}={v}")).collect()
    }

    #[test]
    fn pri_3_2_each_detector_finds_its_values_and_leaves_look_alikes() {
        let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.c2lnbmF0dXJl";
        let found_cases = [
            ("write to priya.k+work@mail.example.co.in today", "[EMAIL_1]=priya.k+work@mail.example.co.in"),
            ("call +44 (20) 7946 0958 now", "[PHONE_1]=+44 (20) 7946 0958"),
            ("call +91 98765 43210", "[PHONE_1]=+91 98765 43210"),
            ("mobile 9876543210.", "[PHONE_1]=9876543210"),
            ("office (555) 123-4567", "[PHONE_1]=(555) 123-4567"),
            ("office 555-123-4567", "[PHONE_1]=555-123-4567"),
            ("card 4111 1111 1111 1111 exp", "[CARD_1]=4111 1111 1111 1111"),
            ("amex 3782-822463-10005", "[CARD_1]=3782-822463-10005"),
            ("iban DE89 3704 0044 0532 0130 00 ok", "[IBAN_1]=DE89 3704 0044 0532 0130 00"),
            ("iban GB82WEST12345698765432", "[IBAN_1]=GB82WEST12345698765432"),
            ("key sk-ant-api03-AbCdEfGhIjKlMnOpQrStUv here", "[APIKEY_1]=sk-ant-api03-AbCdEfGhIjKlMnOpQrStUv"),
            ("key sk-proj-AbCdEfGhIjKlMnOpQrStUv", "[APIKEY_1]=sk-proj-AbCdEfGhIjKlMnOpQrStUv"),
            ("aws AKIAIOSFODNN7EXAMPLE", "[APIKEY_1]=AKIAIOSFODNN7EXAMPLE"),
            ("gh ghp_abcdefghijklmnopqrstuvwxyz0123456789", "[APIKEY_1]=ghp_abcdefghijklmnopqrstuvwxyz0123456789"),
            ("gh github_pat_11ABCDEFG0123456789_abcdefghij", "[APIKEY_1]=github_pat_11ABCDEFG0123456789_abcdefghij"),
            ("slack xoxb-1234567890-abcdef", "[APIKEY_1]=xoxb-1234567890-abcdef"),
            ("google AIzaSyA-abcdefghijklmnopqrstuvwxyz01234", "[APIKEY_1]=AIzaSyA-abcdefghijklmnopqrstuvwxyz01234"),
            ("aadhaar 2345 6789 0124.", "[AADHAAR_1]=2345 6789 0124"),
            ("aadhaar 498765432102", "[AADHAAR_1]=498765432102"),
            ("pan ABCDE1234F", "[PAN_1]=ABCDE1234F"),
        ];
        for (text, want) in found_cases {
            assert_eq!(found(text), [want], "{text}");
        }
        assert_eq!(found(&format!("bearer {jwt}")), [format!("[APIKEY_1]={jwt}")]);
        let look_alikes = [
            "user@localhost and a@b.c",
            "2026-10-13 at 10:30, version 1.2.3, pi 3.14159",
            "order 12345678 and invoice 1234567890",
            "4111 1111 1111 1112",
            "DE89 3704 0044 0532 0130 02", // (with 01 the digits happen to pass Luhn, so they go as a card: safe)
            "sk-short and task-list-items",
            "1234 5678 9012 and 2345 6789 0125",
            "ABCD1234F and abcde1234f",
            "12-34-56 and 555 1234",
        ];
        for text in look_alikes {
            assert_eq!(found(text), Vec::<String>::new(), "{text}");
        }
        // An email made of digits is one email, not a phone number inside one.
        assert_eq!(found("9876543210@example.com"), ["[EMAIL_1]=9876543210@example.com"]);
    }

    #[test]
    fn pri_3_1_a_value_keeps_its_placeholder_and_each_type_counts_from_one() {
        let (text, hidden) = hide(&PrivacySettings::default(), "Mail a@x.io, then b@y.io; card 4111111111111111; again a@x.io.");
        assert_eq!(text, "Mail [EMAIL_1], then [EMAIL_2]; card [CARD_1]; again [EMAIL_1].");
        assert_eq!(hidden.placeholders(), ["[EMAIL_1]", "[EMAIL_2]", "[CARD_1]"]);
    }

    #[test]
    fn pri_3_3_a_streamed_answer_is_restored_wherever_the_pieces_split() {
        let (_, hidden) = hide(&PrivacySettings::default(), "a@x.io b@y.io 4111111111111111");
        let answer = "Write to [EMAIL_1] and [EMAIL_2]; [CARD_1] is a card. [EMAIL_9] is made up. [link](x) [";
        let whole = hidden.restore(answer);
        assert_eq!(whole, "Write to a@x.io and b@y.io; 4111111111111111 is a card. [EMAIL_9] is made up. [link](x) [");
        for cut in (0..=answer.len()).filter(|&i| answer.is_char_boundary(i)) {
            let mut r = Restorer::new(hidden.clone());
            let streamed = r.push(&answer[..cut]) + &r.push(&answer[cut..]) + &r.finish();
            assert_eq!(streamed, whole, "split at {cut}");
        }
        // One character at a time, and nothing held back when nothing was hidden.
        let mut r = Restorer::new(hidden.clone());
        let streamed: String = answer.chars().map(|c| r.push(&c.to_string())).collect::<String>() + &r.finish();
        assert_eq!(streamed, whole);
        assert_eq!(Restorer::new(Hidden::default()).push("[EMA"), "[EMA");
    }

    #[test]
    fn pri_3_4_detectors_switch_off_and_custom_patterns_are_checked() {
        let off = PrivacySettings { detectors_off: vec![Detector::Email], ..PrivacySettings::default() };
        assert_eq!(hide(&off, "a@x.io 9876543210").0, "a@x.io [PHONE_1]");
        let custom = PrivacySettings { patterns: vec![Pattern { label: "emp".into(), regex: r"EMP-\d{6}".into() }, Pattern { label: "Any".into(), regex: "z*".into() }], ..PrivacySettings::default() };
        assert_eq!(validate(&custom), Ok(()));
        assert_eq!(hide(&custom, "badge EMP-123456 of a@x.io").0, "badge [CUSTOM_EMP_1] of [EMAIL_1]", "empty matches are skipped");
        let bad = |label: &str, regex: &str| validate(&PrivacySettings { patterns: vec![Pattern { label: label.into(), regex: regex.into() }], ..PrivacySettings::default() }).unwrap_err();
        assert!(bad("emp", "EMP-(\\d").contains("isn't a valid regular expression"), "{}", bad("emp", "EMP-(\\d"));
        assert!(bad("my label", "x").contains("letters or digits"));
        assert!(bad("", "x").contains("letters or digits"));
        assert!(Detectors::new(&PrivacySettings { patterns: vec![Pattern { label: "x".into(), regex: "(".into() }], ..PrivacySettings::default() }).is_err(), "a bad pattern never sends unredacted");
    }

    #[test]
    fn pri_3_5_local_and_lan_models_are_redacted_only_when_asked() {
        let on = PrivacySettings::default();
        assert!(applies(&on, Locality::Cloud));
        assert!(!applies(&on, Locality::Local) && !applies(&on, Locality::Lan));
        let local_too = PrivacySettings { redact_local: true, ..on.clone() };
        assert!(applies(&local_too, Locality::Local) && applies(&local_too, Locality::Lan));
        let off = PrivacySettings { redact: false, redact_local: true, ..on };
        assert!(!applies(&off, Locality::Cloud) && !applies(&off, Locality::Local), "the main switch turns it all off");
    }

    #[test]
    fn pri_3_7_a_cloud_request_waits_for_send_while_confirm_is_on() {
        let confirm = PrivacySettings { confirm_cloud: true, ..PrivacySettings::default() };
        assert!(must_confirm(&confirm, Locality::Cloud, false));
        assert!(!must_confirm(&confirm, Locality::Cloud, true));
        assert!(!must_confirm(&confirm, Locality::Local, false) && !must_confirm(&confirm, Locality::Lan, false));
        assert!(!must_confirm(&PrivacySettings::default(), Locality::Cloud, false));
    }

    #[test]
    fn pri_3_8_an_image_for_a_cloud_model_is_warned_about() {
        let mut with_image = Message::new(Role::User, "what is this?");
        with_image.images.push(Image { mime: "image/png".into(), base64: "AAAA".into() });
        let text_only = [Message::new(Role::User, "hi")];
        assert!(image_warning(std::slice::from_ref(&with_image), Locality::Cloud));
        assert!(!image_warning(&[with_image], Locality::Local));
        assert!(!image_warning(&text_only, Locality::Cloud));
    }
}

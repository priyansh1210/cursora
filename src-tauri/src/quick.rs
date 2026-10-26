//! Quick actions (spec 14): the bubble's one-click buttons, chosen by the kind of app the text came from.
use crate::capture::SourceApp;
use crate::skills::Skill;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Terminal,
    Code,
    Mail,
    Other,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct QuickAction {
    pub label: String,
    pub command: String,
}

// shortcut: fixed tables; an app they don't know counts as Other. Add names as users report them.
const TERMINALS: [&str; 11] = [
    "windowsterminal.exe", "alacritty.exe", "wezterm-gui.exe", "kitty", "alacritty", "wezterm-gui", "gnome-terminal-server", "konsole", "xterm",
    "xfce4-terminal", "tilix",
];
const CODE: [&str; 14] = [
    "code.exe", "code", "cursor.exe", "cursor", "devenv.exe", "idea64.exe", "pycharm64.exe", "webstorm64.exe", "clion64.exe", "rider64.exe",
    "notepad++.exe", "sublime_text.exe", "sublime_text", "zed.exe",
];
const MAIL: [&str; 16] = [
    "outlook.exe", "olk.exe", "thunderbird.exe", "thunderbird", "ms-teams.exe", "teams.exe", "slack.exe", "slack", "discord.exe", "discord",
    "whatsapp.exe", "telegram.exe", "telegram-desktop", "signal.exe", "signal-desktop", "evolution",
];

/// QAC-2.1: the user's own "Ctrl+C may stop a process" list counts as terminals too. Editors come first: that list has
/// Code.exe and the JetBrains IDEs for their built-in terminals.
pub fn kind(app: &SourceApp, unsafe_apps: &[String]) -> Kind {
    let exe = app.exe.to_lowercase();
    let is = |list: &[&str]| list.contains(&exe.as_str());
    if is(&CODE) {
        Kind::Code
    } else if is(&MAIL) {
        Kind::Mail
    } else if is(&TERMINALS) || unsafe_apps.iter().any(|a| a.eq_ignore_ascii_case(&exe)) {
        Kind::Terminal
    } else {
        Kind::Other
    }
}

/// QAC-2.2.
pub fn set(kind: Kind) -> &'static [(&'static str, &'static str)] {
    match kind {
        Kind::Terminal => &[("Explain error", "/error"), ("Explain command", "/code"), ("Explain", "/explain")],
        Kind::Code => &[("Explain", "/code"), ("Find bugs", "/bugs"), ("Explain error", "/error"), ("Summarize", "/summary")],
        Kind::Mail => &[
            ("Reply", "/reply"),
            ("Fix", "/fix"),
            ("More polite", "/rewrite more polite"),
            ("Shorter", "/rewrite shorter"),
            ("Translate", "/translate"),
            ("Explain", "/explain"),
        ],
        Kind::Other => &[
            ("Fix", "/fix"),
            ("Rewrite", "/rewrite"),
            ("Shorter", "/rewrite shorter"),
            ("Explain", "/explain"),
            ("Summarize", "/summary"),
            ("Translate", "/translate"),
        ],
    }
}

/// Pinned skills first, then the kind's set; only skills that exist and can run; no duplicates; at most six (QAC-1.3, 3.1).
/// `can_see`: the skill's model takes images, as in `skills::entries`.
pub fn actions(kind: Kind, pinned: &[String], skills: &[Skill], can_see: impl Fn(&Skill) -> bool) -> Vec<QuickAction> {
    let pinned = pinned.iter().map(|name| {
        let mut c = name.chars();
        (c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default(), format!("/{name}"))
    });
    let set = set(kind).iter().map(|(label, command)| (label.to_string(), command.to_string()));
    let runs = |command: &str| {
        let name = command.trim_start_matches('/').split(' ').next().unwrap_or_default();
        skills.iter().any(|s| s.name == name && (!s.vision || can_see(s)))
    };
    let mut out: Vec<QuickAction> = Vec::new();
    for (label, command) in pinned.chain(set) {
        if out.len() < 6 && runs(&command) && !out.iter().any(|a| a.command == command) {
            out.push(QuickAction { label, command });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(exe: &str) -> SourceApp {
        SourceApp { exe: exe.into(), ..SourceApp::default() }
    }

    #[test]
    fn qac_2_1_the_kind_of_app_comes_from_its_program_name() {
        let unsafe_apps = crate::settings::CaptureSettings::default().unsafe_apps;
        assert_eq!(kind(&app("WindowsTerminal.exe"), &unsafe_apps), Kind::Terminal);
        assert_eq!(kind(&app("konsole"), &unsafe_apps), Kind::Terminal);
        assert_eq!(kind(&app("cmd.exe"), &unsafe_apps), Kind::Terminal, "from the default list");
        assert_eq!(kind(&app("myshell.exe"), &["MyShell.exe".into()]), Kind::Terminal, "the user's own list counts");
        assert_eq!(kind(&app("Code.exe"), &unsafe_apps), Kind::Code, "in the list for its built-in terminal, still an editor");
        assert_eq!(kind(&app("idea64.exe"), &unsafe_apps), Kind::Code);
        assert_eq!(kind(&app("OUTLOOK.EXE"), &unsafe_apps), Kind::Mail);
        assert_eq!(kind(&app("slack"), &unsafe_apps), Kind::Mail);
        assert_eq!(kind(&app("chrome.exe"), &unsafe_apps), Kind::Other, "a browser, even showing webmail");
        assert_eq!(kind(&app(""), &unsafe_apps), Kind::Other);
    }

    #[test]
    fn qac_2_2_each_kind_has_its_buttons() {
        let commands = |k| set(k).iter().map(|(_, c)| *c).collect::<Vec<_>>();
        assert_eq!(commands(Kind::Terminal), ["/error", "/code", "/explain"]);
        assert_eq!(commands(Kind::Code), ["/code", "/bugs", "/error", "/summary"]);
        assert_eq!(commands(Kind::Mail), ["/reply", "/fix", "/rewrite more polite", "/rewrite shorter", "/translate", "/explain"]);
        assert_eq!(commands(Kind::Other), ["/fix", "/rewrite", "/rewrite shorter", "/explain", "/summary", "/translate"]);
    }

    #[test]
    fn qac_1_3_a_button_needs_its_skill_and_what_the_skill_requires() {
        let (mut skills, _) = crate::skills::load(None);
        skills.retain(|s| s.name != "summary");
        let named = |a: &[QuickAction]| a.iter().map(|a| a.command.clone()).collect::<Vec<_>>();
        assert_eq!(named(&actions(Kind::Code, &[], &skills, |_| false)), ["/code", "/bugs", "/error"], "no /summary skill, no Summarize");
        skills.push(crate::skills::parse("---\nname: look\ndescription: d\nrequires: [vision]\n---\nLook.").unwrap());
        assert!(!named(&actions(Kind::Other, &["look".into()], &skills, |_| false)).contains(&"/look".to_string()), "the model can't see");
        assert_eq!(named(&actions(Kind::Other, &["look".into()], &skills, |_| true))[0], "/look");
    }

    #[test]
    fn qac_3_1_pinned_skills_come_first_without_duplicates_and_six_at_most() {
        let (skills, _) = crate::skills::load(None);
        let a = actions(Kind::Other, &["define".into(), "fix".into()], &skills, |_| false);
        let shown: Vec<(&str, &str)> = a.iter().map(|a| (a.label.as_str(), a.command.as_str())).collect();
        assert_eq!(shown, [("Define", "/define"), ("Fix", "/fix"), ("Rewrite", "/rewrite"), ("Shorter", "/rewrite shorter"), ("Explain", "/explain"), ("Summarize", "/summary")]);
        assert!(actions(Kind::Other, &["gone".into()], &skills, |_| false).iter().all(|a| a.command != "/gone"), "a pinned skill that was deleted");
    }

    #[test]
    fn qac_4_1_reply_drafts_an_answer_in_the_chat() {
        let (skills, _) = crate::skills::load(None);
        let reply = skills.iter().find(|s| s.name == "reply").expect("a built-in");
        assert!(!reply.replace, "the selection is the message received, not text to replace");
        assert!(reply.template.contains("language") && reply.template.contains("{{input}}"));
    }

    #[test]
    fn qac_4_2_error_explains_the_error_and_the_fix_in_steps() {
        let (skills, _) = crate::skills::load(None);
        let error = skills.iter().find(|s| s.name == "error").expect("a built-in");
        assert!(!error.replace && !error.vision);
        assert!(error.template.contains("numbered steps"));
    }
}

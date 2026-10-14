#[derive(Debug, PartialEq)]
pub enum Action {
    Ask,
    Snip,
    Settings,
    Quit,
}

/// Arguments without the program name. Unknown ones are logged and ignored (APP-2.1).
pub fn parse<S: AsRef<str>>(args: &[S]) -> Vec<Action> {
    args.iter()
        .filter_map(|a| match a.as_ref() {
            "--ask" => Some(Action::Ask),
            "--snip" => Some(Action::Snip),
            "--settings" => Some(Action::Settings),
            "--quit" => Some(Action::Quit),
            a if a.starts_with(crate::admin::AFTER) => None, // read before the app starts (APP-3.4)
            other => {
                log::warn!("ignoring unknown argument {other:?}");
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_2_1_parses_known_args_and_ignores_unknown() {
        assert_eq!(
            parse(&["--ask", "--bogus", "--settings", "--snip", "--quit"]),
            vec![Action::Ask, Action::Settings, Action::Snip, Action::Quit]
        );
        assert!(parse::<&str>(&[]).is_empty());
    }
}

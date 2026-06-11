use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Binding {
    keys: Vec<String>,
    help: Help,
    disabled: bool,
}

pub type BindingOpt = Box<dyn Fn(&mut Binding)>;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Help {
    pub key: String,
    pub desc: String,
}

impl Binding {
    pub fn new(opts: impl IntoIterator<Item = BindingOpt>) -> Self {
        let mut binding = Self::default();
        for opt in opts {
            opt(&mut binding);
        }
        binding
    }

    pub fn set_keys(&mut self, keys: impl IntoIterator<Item = impl Into<String>>) {
        self.keys = keys.into_iter().map(Into::into).collect();
    }

    pub fn keys(&self) -> &[String] {
        &self.keys
    }

    pub fn set_help(&mut self, key: impl Into<String>, desc: impl Into<String>) {
        self.help = Help {
            key: key.into(),
            desc: desc.into(),
        };
    }

    pub fn help(&self) -> &Help {
        &self.help
    }

    pub fn enabled(&self) -> bool {
        !self.disabled && !self.keys.is_empty()
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.disabled = !enabled;
    }

    pub fn unbind(&mut self) {
        self.keys.clear();
        self.help = Help::default();
    }
}

pub fn with_keys(keys: &'static [&'static str]) -> BindingOpt {
    Box::new(move |binding| {
        binding.keys = keys.iter().map(|k| (*k).to_string()).collect();
    })
}

pub fn with_help(key: &'static str, desc: &'static str) -> BindingOpt {
    Box::new(move |binding| {
        binding.help = Help {
            key: key.to_string(),
            desc: desc.to_string(),
        };
    })
}

pub fn with_disabled() -> BindingOpt {
    Box::new(|binding| binding.disabled = true)
}

pub fn matches<'a, I>(event: &KeyEvent, bindings: I) -> bool
where
    I: IntoIterator<Item = &'a Binding>,
{
    let key = key_event_to_string(event);
    bindings
        .into_iter()
        .filter(|binding| binding.enabled())
        .any(|binding| binding.keys.iter().any(|candidate| candidate == &key))
}

pub fn key_event_to_string(event: &KeyEvent) -> String {
    let mut parts = Vec::new();
    if event.modifiers.contains(KeyModifiers::CONTROL) {
        parts.push("ctrl".to_string());
    }
    if event.modifiers.contains(KeyModifiers::ALT) {
        parts.push("alt".to_string());
    }
    if event.modifiers.contains(KeyModifiers::SHIFT)
        && !matches!(event.code, KeyCode::Char(c) if c.is_ascii_uppercase())
    {
        parts.push("shift".to_string());
    }

    let code = match event.code {
        KeyCode::Backspace => "backspace".to_string(),
        KeyCode::Enter => "enter".to_string(),
        KeyCode::Left => "left".to_string(),
        KeyCode::Right => "right".to_string(),
        KeyCode::Up => "up".to_string(),
        KeyCode::Down => "down".to_string(),
        KeyCode::Home => "home".to_string(),
        KeyCode::End => "end".to_string(),
        KeyCode::PageUp => "pgup".to_string(),
        KeyCode::PageDown => "pgdown".to_string(),
        KeyCode::Tab => "tab".to_string(),
        KeyCode::BackTab => "shift+tab".to_string(),
        KeyCode::Delete => "delete".to_string(),
        KeyCode::Insert => "insert".to_string(),
        KeyCode::Esc => "esc".to_string(),
        KeyCode::Char(c) => c.to_ascii_lowercase().to_string(),
        KeyCode::F(n) => format!("f{n}"),
        _ => format!("{:?}", event.code).to_ascii_lowercase(),
    };

    if code == "shift+tab" {
        return code;
    }

    parts.push(code);
    parts.join("+")
}

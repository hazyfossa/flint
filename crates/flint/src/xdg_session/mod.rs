use anyhow::{Context, Result, bail, ensure};
use envy::define_env;

use std::{
    fs::File,
    io::{BufRead, BufReader, Lines, Read},
    path::PathBuf,
};

use crate::with_builder;

const X11_SESSION_PATH: &str = "/usr/share/xsessions";
const WAYLAND_SESSION_PATH: &str = "/usr/share/wayland-sessions";

// pub struct LocaleString {
//     default: String,
//     lc_lookup: HashMap<String, String>,
// }

type LocaleString = String;

// https://www.freedesktop.org/software/systemd/man/latest/pam_systemd.html#type=
define_env!(SessionKind = "XDG_SESSION_TYPE");

crate::strenum!(
    #[derive(Debug, PartialEq, Eq)]
    pub SessionKind {
        Unspecified,
        TTY,
        X11,
        Wayland,
        Mir,
        Web,
    }
);

// TODO: where are those session-entry-types specified?
pub enum KindHint {
    X11,
    Any,
}

with_builder!(
    pub struct SessionEntry {
        pub name: #required LocaleString,
        pub kind_hint: #required KindHint,
        // NOTE: deviating the spec, this is required, as a session is never DBusActivatable
        pub executable: #required PathBuf,
        pub working_directory: PathBuf,
        // TODO: does it make sense to check TryExec?
        pub comment: LocaleString,
        pub desktop_names: DesktopList,
    }
);

// https://www.freedesktop.org/software/systemd/man/latest/pam_systemd.html#desktop=
define_env!(pub Desktop(String) = "XDG_SESSION_DESKTOP");

// https://specifications.freedesktop.org/desktop-entry/latest/recognized-keys.html#id-1.7.6
// see: OnlyShowIn, NotShowIn
define_env!(pub DesktopList(String) = "XDG_CURRENT_DESKTOP");

impl DesktopList {
    pub fn as_single_desktop(&self) -> Option<Desktop> {
        match &self.0.contains(";") {
            true => None,
            false => Some(Desktop(self.0.clone())),
        }
    }

    pub fn to_vec(self) -> Vec<String> {
        self.split(";").map(String::from).collect()
    }
}

enum ParserState {
    Continue,
    EOF,
}

struct Parser<R> {
    builder: SessionEntryBuilder,
    reader: Lines<BufReader<R>>,
}

impl<R: Read> Parser<R> {
    fn new(reader: BufReader<R>) -> Self {
        Self {
            builder: SessionEntryBuilder::new(),
            reader: reader.lines(),
        }
    }

    fn read_next(&mut self) -> Result<ParserState> {
        // read the next non-empty, non-comment line
        let line = loop {
            let line = match self.reader.next() {
                Some(line) => line?,
                None => return Ok(ParserState::EOF),
            };

            let skip = line.is_empty() || line.starts_with("#");

            match skip {
                true => continue,
                false => break line,
            }
        };

        ensure!(
            !line.starts_with("["),
            "This parser does not support groups"
        );

        let (k, v) = line
            .split_once("=")
            .context("Cannot parse as a key-value pair: cannot split at =")?;

        let (k, v) = (k.trim_end(), v.trim_start());

        match k {
            "Type" => self.builder.set_kind_hint(match v {
                "Application" => KindHint::Any,
                "XSession" => KindHint::X11,
                other => bail!("Unsupported entry kind: {other}"),
            }),

            "Exec" => self.builder.set_executable(v.into()),
            "Path" => self.builder.set_working_directory(v.into()),
            "Name" => self.builder.set_name(v.to_string()),
            "Comment" => self.builder.set_comment(v.to_string()),
            "DesktopNames" => self.builder.set_desktop_names(DesktopList(v.to_string())),

            _skip_other => return Ok(ParserState::Continue),
        };

        Ok(ParserState::Continue)
    }

    fn read_all(mut self) -> Result<SessionEntry> {
        loop {
            match self.read_next() {
                Ok(ParserState::Continue) => (),
                Ok(ParserState::EOF) => break,
                Err(e) => return Err(e),
            }
        }

        self.builder.finalize()
    }
}

pub fn parse(reader: BufReader<impl Read>) -> Result<SessionEntry> {
    Parser::new(reader).read_all()
}

pub fn get_session_entry(kind: SessionKind, name: &str) -> Result<SessionEntry> {
    let path: PathBuf = match kind {
        SessionKind::X11 => X11_SESSION_PATH,
        SessionKind::Wayland => WAYLAND_SESSION_PATH,
        other => panic!("There is no standart location for {other} session"),
    }
    .into();

    let path = path.join(format!("{name}.desktop"));

    let file = File::open(&path)
        .with_context(|| format!("Failed to read session definition file: {path:?}"))?;

    parse(BufReader::with_capacity(4096, file))
}

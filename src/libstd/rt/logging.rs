// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
use cast::transmute;
use either::*;
use libc::{c_void, uintptr_t, c_char, exit, STDERR_FILENO};
use option::{Some, None, Option};
use rt::util::dumb_println;
use str::StrSlice;
use str::raw::from_c_str;
use u32;
use unstable::raw::Closure;
use vec::ImmutableVector;


struct LogDirective {
    name: Option<~str>,
    level: u32
}

// This is the Rust representation of the mod_entry struct in src/rt/rust_crate_map.h
struct ModEntry{
    name: *c_char,
    log_level: *mut u32
}

static MAX_LOG_LEVEL: u32 = 255;
static DEFAULT_LOG_LEVEL: u32 = 1;

fn iter_crate_map(map: *u8, f: &fn(*mut ModEntry)) {
    unsafe {
        let closure : Closure = transmute(f);
        let code = transmute(closure.code);
        let env = transmute(closure.env);
        rust_iter_crate_map(transmute(map), iter_cb, code, env);
    }

    extern fn iter_cb(code: *c_void, env: *c_void, entry: *ModEntry){
         unsafe {
            let closure: Closure = Closure {
                code: transmute(code),
                env: transmute(env),
            };
            let closure: &fn(*ModEntry) = transmute(closure);
            return closure(entry);
        }
    }
    extern {
        #[cfg(not(stage0))]
        #[rust_stack]
        fn rust_iter_crate_map(map: *c_void,
                    f: extern "C" fn(*c_void, *c_void, entry: *ModEntry),
                    code: *c_void,
                    data: *c_void);

        #[cfg(stage0)]
        #[rust_stack]
        fn rust_iter_crate_map(map: *c_void,
                    f: *u8,
                    code: *c_void,
                    data: *c_void);
    }
}
static log_level_names : &'static[&'static str] = &'static["error", "warn", "info", "debug"];

/// Parse an individual log level that is either a number or a symbolic log level
fn parse_log_level(level: &str) -> Option<u32> {
    let num = u32::from_str(level);
    let mut log_level;
    match num {
        Some(num) => {
            if num < MAX_LOG_LEVEL {
                log_level = Some(num);
            } else {
                log_level = Some(MAX_LOG_LEVEL);
            }
        }
        _ => {
            let position = log_level_names.iter().position(|&name| name == level);
            match position {
                Some(position) => {
                    log_level = Some(u32::min(MAX_LOG_LEVEL, (position + 1) as u32))
                },
                _ => {
                    log_level = None;
                }
            }
        }
    }
    log_level
}


/// Parse a logging specification string (e.g: "crate1,crate2::mod3,crate3::x=1")
/// and return a vector with log directives.
/// Valid log levels are 0-255, with the most likely ones being 1-4 (defined in std::).
/// Also supports string log levels of error, warn, info, and debug

fn parse_logging_spec(spec: ~str) -> ~[LogDirective]{
    let mut dirs = ~[];
    for s in spec.split_iter(',') {
        let parts: ~[&str] = s.split_iter('=').collect();
        let mut log_level;
        let mut name = Some(parts[0].to_owned());
        match parts.len() {
            1 => {
                //if the single argument is a log-level string or number,
                //treat that as a global fallback
                let possible_log_level = parse_log_level(parts[0]);
                match possible_log_level {
                    Some(num) => {
                        name = None;
                        log_level = num;
                    },
                    _ => {
                        log_level = MAX_LOG_LEVEL
                    }
                }
            }
            2 => {
                let possible_log_level = parse_log_level(parts[1]);
                match possible_log_level {
                    Some(num) => {
                        log_level = num;
                    },
                    _ => {
                        dumb_println(fmt!("warning: invalid logging spec \
                                           '%s', ignoring it", parts[1]));
                        loop;
                    }
                }
            },
            _ => {
                dumb_println(fmt!("warning: invalid logging spec '%s',\
                                  ignoring it", s));
                loop;
            }
        }
        let dir = LogDirective {name: name, level: log_level};
        dirs.push(dir);
    }
    return dirs;
}

/// Set the log level of an entry in the crate map depending on the vector
/// of log directives
fn update_entry(dirs: &[LogDirective], entry: *mut ModEntry) -> u32 {
    let mut new_lvl: u32 = DEFAULT_LOG_LEVEL;
    let mut longest_match = -1i;
    unsafe {
        for dir in dirs.iter() {
            match dir.name {
                None => {
                    if longest_match == -1 {
                        longest_match = 0;
                        new_lvl = dir.level;
                    }
                }
                Some(ref dir_name) => {
                    let name = from_c_str((*entry).name);
                    let len = dir_name.len() as int;
                    if name.starts_with(*dir_name) &&
                        len >= longest_match {
                        longest_match = len;
                        new_lvl = dir.level;
                    }
                }
            };
        }
        *(*entry).log_level = new_lvl;
    }
    if longest_match >= 0 { return 1; } else { return 0; }
}

#[fixed_stack_segment] #[inline(never)]
/// Set log level for every entry in crate_map according to the sepecification
/// in settings
fn update_log_settings(crate_map: *u8, settings: ~str) {
    let mut dirs = ~[];
    if settings.len() > 0 {
        if settings == ~"::help" || settings == ~"?" {
            dumb_println("\nCrate log map:\n");
            do iter_crate_map(crate_map) |entry: *mut ModEntry| {
                unsafe {
                    dumb_println(" "+from_c_str((*entry).name));
                }
            }
            unsafe {
                exit(1);
            }
        }
        dirs = parse_logging_spec(settings);
    }

    let mut n_matches: u32 = 0;
    do iter_crate_map(crate_map) |entry: *mut ModEntry| {
        let m = update_entry(dirs, entry);
        n_matches += m;
    }

    if n_matches < (dirs.len() as u32) {
        dumb_println(fmt!("warning: got %u RUST_LOG specs but only matched %u of them.\n\
                          You may have mistyped a RUST_LOG spec.\n\
                          Use RUST_LOG=::help to see the list of crates and modules.\n",
                          dirs.len() as uint, n_matches as uint));
    }
}

pub trait Logger {
    fn log(&mut self, msg: Either<~str, &'static str>);
}

pub struct StdErrLogger;

impl Logger for StdErrLogger {
    fn log(&mut self, msg: Either<~str, &'static str>) {
        use io::{Writer, WriterUtil};

        if !should_log_console() {
            return;
        }

        let s: &str = match msg {
            Left(ref s) => {
                let s: &str = *s;
                s
            }
            Right(ref s) => {
                let s: &str = *s;
                s
            }
        };

        // Truncate the string
        let buf_bytes = 2048;
        if s.len() > buf_bytes {
            let s = s.slice(0, buf_bytes) + "[...]";
            print(s);
        } else {
            print(s)
        };

        fn print(s: &str) {
            let dbg = STDERR_FILENO as ::io::fd_t;
            dbg.write_str(s);
            dbg.write_str("\n");
            dbg.flush();
        }
    }
}
/// Configure logging by traversing the crate map and setting the
/// per-module global logging flags based on the logging spec
#[fixed_stack_segment] #[inline(never)]
pub fn init(crate_map: *u8) {
    use os;

    let log_spec = os::getenv("RUST_LOG");
    match log_spec {
        Some(spec) => {
            update_log_settings(crate_map, spec);
        }
        None => {
            update_log_settings(crate_map, ~"");
        }
    }
}

#[fixed_stack_segment] #[inline(never)]
pub fn console_on() { unsafe { rust_log_console_on() } }

#[fixed_stack_segment] #[inline(never)]
pub fn console_off() { unsafe { rust_log_console_off() } }

#[fixed_stack_segment] #[inline(never)]
fn should_log_console() -> bool { unsafe { rust_should_log_console() != 0 } }

extern {
    fn rust_log_console_on();
    fn rust_log_console_off();
    fn rust_should_log_console() -> uintptr_t;
}

// Tests for parse_logging_spec()
#[test]
fn parse_logging_spec_valid() {
    let dirs = parse_logging_spec(~"crate1::mod1=1,crate1::mod2,crate2=4");
    assert_eq!(dirs.len(), 3);
    assert!(dirs[0].name == Some(~"crate1::mod1"));
    assert_eq!(dirs[0].level, 1);

    assert!(dirs[1].name == Some(~"crate1::mod2"));
    assert_eq!(dirs[1].level, MAX_LOG_LEVEL);

    assert!(dirs[2].name == Some(~"crate2"));
    assert_eq!(dirs[2].level, 4);
}

#[test]
fn parse_logging_spec_invalid_crate() {
    // test parse_logging_spec with multiple = in specification
    let dirs = parse_logging_spec(~"crate1::mod1=1=2,crate2=4");
    assert_eq!(dirs.len(), 1);
    assert!(dirs[0].name == Some(~"crate2"));
    assert_eq!(dirs[0].level, 4);
}

#[test]
fn parse_logging_spec_invalid_log_level() {
    // test parse_logging_spec with 'noNumber' as log level
    let dirs = parse_logging_spec(~"crate1::mod1=noNumber,crate2=4");
    assert_eq!(dirs.len(), 1);
    assert!(dirs[0].name == Some(~"crate2"));
    assert_eq!(dirs[0].level, 4);
}

#[test]
fn parse_logging_spec_string_log_level() {
    // test parse_logging_spec with 'warn' as log level
    let dirs = parse_logging_spec(~"crate1::mod1=wrong,crate2=warn");
    assert_eq!(dirs.len(), 1);
    assert!(dirs[0].name == Some(~"crate2"));
    assert_eq!(dirs[0].level, 2);
}

#[test]
fn parse_logging_spec_global() {
    // test parse_logging_spec with no crate
    let dirs = parse_logging_spec(~"warn,crate2=4");
    assert_eq!(dirs.len(), 2);
    assert!(dirs[0].name == None);
    assert_eq!(dirs[0].level, 2);
    assert!(dirs[1].name == Some(~"crate2"));
    assert_eq!(dirs[1].level, 4);
}

// Tests for update_entry
#[test]
fn update_entry_match_full_path() {
    use c_str::ToCStr;
    let dirs = ~[LogDirective {name: Some(~"crate1::mod1"), level: 2 },
                 LogDirective {name: Some(~"crate2"), level: 3}];
    let level = &mut 0;
    unsafe {
        do "crate1::mod1".to_c_str().with_ref |ptr| {
            let entry= &ModEntry {name: ptr, log_level: level};
            let m = update_entry(dirs, transmute(entry));
            assert!(*entry.log_level == 2);
            assert!(m == 1);
        }
    }
}

#[test]
fn update_entry_no_match() {
    use c_str::ToCStr;
    let dirs = ~[LogDirective {name: Some(~"crate1::mod1"), level: 2 },
                 LogDirective {name: Some(~"crate2"), level: 3}];
    let level = &mut 0;
    unsafe {
        do "crate3::mod1".to_c_str().with_ref |ptr| {
            let entry= &ModEntry {name: ptr, log_level: level};
            let m = update_entry(dirs, transmute(entry));
            assert!(*entry.log_level == DEFAULT_LOG_LEVEL);
            assert!(m == 0);
        }
    }
}

#[test]
fn update_entry_match_beginning() {
    use c_str::ToCStr;
    let dirs = ~[LogDirective {name: Some(~"crate1::mod1"), level: 2 },
                 LogDirective {name: Some(~"crate2"), level: 3}];
    let level = &mut 0;
    unsafe {
        do "crate2::mod1".to_c_str().with_ref |ptr| {
            let entry= &ModEntry {name: ptr, log_level: level};
            let m = update_entry(dirs, transmute(entry));
            assert!(*entry.log_level == 3);
            assert!(m == 1);
        }
    }
}

#[test]
fn update_entry_match_beginning_longest_match() {
    use c_str::ToCStr;
    let dirs = ~[LogDirective {name: Some(~"crate1::mod1"), level: 2 },
                 LogDirective {name: Some(~"crate2"), level: 3},
                 LogDirective {name: Some(~"crate2::mod"), level: 4}];
    let level = &mut 0;
    unsafe {
        do "crate2::mod1".to_c_str().with_ref |ptr| {
            let entry = &ModEntry {name: ptr, log_level: level};
            let m = update_entry(dirs, transmute(entry));
            assert!(*entry.log_level == 4);
            assert!(m == 1);
        }
    }
}

#[test]
fn update_entry_match_default() {
    use c_str::ToCStr;
    let dirs = ~[LogDirective {name: Some(~"crate1::mod1"), level: 2 },
                 LogDirective {name: None, level: 3}
                ];
    let level = &mut 0;
    unsafe {
        do "crate1::mod1".to_c_str().with_ref |ptr| {
            let entry= &ModEntry {name: ptr, log_level: level};
            let m = update_entry(dirs, transmute(entry));
            assert!(*entry.log_level == 2);
            assert!(m == 1);
        }
        do "crate2::mod2".to_c_str().with_ref |ptr| {
            let entry= &ModEntry {name: ptr, log_level: level};
            let m = update_entry(dirs, transmute(entry));
            assert!(*entry.log_level == 3);
            assert!(m == 1);
        }
    }
}

// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::io;

pub fn general() {
    io::println("Usage: rustpkg [options] <cmd> [args..]

Where <cmd> is one of:
    build, clean, do, info, install, list, prefer, test, uninstall, unprefer

Options:

    -h, --help                  Display this message
    <cmd> -h, <cmd> --help      Display help for <cmd>");
}

pub fn build() {
    io::println("rustpkg [options..] build [package-ID]

Build the given package ID if specified. With no package ID argument,
build the package in the current directory. In that case, the current
directory must be a direct child of an `src` directory in a workspace.

Options:
    -c, --cfg      Pass a cfg flag to the package script");
}

pub fn clean() {
    io::println("rustpkg clean

Remove all build files in the work cache for the package in the current
directory.");
}

pub fn do_cmd() {
    io::println("rustpkg do <cmd>

Runs a command in the package script. You can listen to a command
by tagging a function with the attribute `#[pkg_do(cmd)]`.");
}

pub fn info() {
    io::println("rustpkg [options..] info

Probe the package script in the current directory for information.

Options:
    -j, --json      Output the result as JSON");
}

pub fn list() {
    io::println("rustpkg list

List all installed packages.");
}

pub fn install() {
    io::println("rustpkg [options..] install [package-ID]

Install the given package ID if specified. With no package ID
argument, install the package in the current directory.
In that case, the current directory must be a direct child of a
`src` directory in a workspace.

Examples:
    rustpkg install
    rustpkg install github.com/mozilla/servo
    rustpkg install github.com/mozilla/servo#0.1.2

Options:
    -c, --cfg      Pass a cfg flag to the package script");
}

pub fn uninstall() {
    io::println("rustpkg uninstall <id|name>[@version]

Remove a package by id or name and optionally version. If the package(s)
is/are depended on by another package then they cannot be removed.");
}

pub fn prefer() {
    io::println("rustpkg [options..] prefer <id|name>[@version]

By default all binaries are given a unique name so that multiple versions can
coexist. The prefer command will symlink the uniquely named binary to
the binary directory under its bare name. If version is not supplied, the
latest version of the package will be preferred.

Example:
    export PATH=$PATH:/home/user/.rustpkg/bin
    rustpkg prefer machine@1.2.4
    machine -v
    ==> v1.2.4
    rustpkg prefer machine@0.4.6
    machine -v
    ==> v0.4.6");
}

pub fn unprefer() {
    io::println("rustpkg [options..] unprefer <id|name>[@version]

Remove all symlinks from the store to the binary directory for a package
name and optionally version. If version is not supplied, the latest version
of the package will be unpreferred. See `rustpkg prefer -h` for more
information.");
}

pub fn test() {
    io::println("rustpkg [options..] test

Build all targets described in the package script in the current directory
with the test flag. The test bootstraps will be run afterwards and the output
and exit code will be redirected.

Options:
    -c, --cfg      Pass a cfg flag to the package script");
}

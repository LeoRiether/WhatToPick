use std::{
    env,
    collections::HashSet,
    ffi::{OsStr, OsString},
    path::Path,
    fs::{self, File},
    io::{BufRead, BufReader},
    process,
    error::Error,
};

use inquire::Select;

const HELP_MESSAGE: &'static str = concat!("
wtp - What To Pick?
Decision trees to help humans decide stuff
v", env!("CARGO_PKG_VERSION"), "

USAGE:
    wtp [FLAG] PICK_TREE_ID

FLAG:
    -h, --help  Shows this message
    -e, --edit  Edits the PICK_TREE_ID file
    -f, --file  Outputs the path for the PICK_TREE_ID file
    no flag     Interactively helps you pick one of the options

PICK_TREE file format:
    It's a tree where siblings are in the same indentation level and children
    have more indentation than their parents. Example:

    A node in level 1
        Some child in level 2
        Another child in level 2
    Another node in level 1
        A child of the node above
        Another child of that same node
        Yet another child of that node
            A node in level 3
            Another one in level 3

    TODO: explain how it works better. I'll do it later. Probably...
");

fn nonempty_env_var<K: AsRef<OsStr>>(k: K) -> Option<String> {
    env::var(k).ok().filter(|x| !x.is_empty())
}

fn editor_command() -> OsString {
    let default = if cfg!(windows) { "notepad" } else { "nano" };

    nonempty_env_var("EDITOR")
        .or(nonempty_env_var("VISUAL"))
        .unwrap_or_else(|| default.into())
        .into()
}

fn spawn_editor(file: &Path) -> Result<(), Box<dyn Error>> {
    process::Command::new(editor_command())
        .arg(file)
        .spawn()?
        .wait()?;
    Ok(())
}

/// Reads the env::args and returns a pair (set of flags, pick tree identifier)
fn args() -> (HashSet<String>, Option<String>) {
    let mut flags = HashSet::new();
    let mut id = None;
    for arg in env::args().skip(1) {
        if arg.starts_with("-") {
            flags.insert(arg);
        } else {
            id = Some(arg);
        }
    }
    (flags, id)
}

struct Tree {
    key: String,
    children: Vec<Tree>,
}

impl Tree {
    pub fn new(key: String) -> Self {
        Self { key, children: Vec::new() }
    }

    pub fn from_file(file: &Path) -> Self {
        let file = File::open(file).expect(&format!("Couldn't open file <{}>", file.to_string_lossy()));
        let reader = BufReader::new(file);

        // Start a stack of parent nodes
        // Every item in the stack is a pair (node, indentation level)
        let mut parents = vec![ (Tree::new("".into()), -1) ];
        for line in reader.lines() {
            let line = line.unwrap();
            let line = line.as_str();

            // count whitespace characters before
            let ws = line.chars().take_while(|c| c.is_whitespace()).count();

            if !line[ws..].is_empty() {
                let node = Tree::new(line[ws..].into());

                // Remove nodes that aren't ancestors of `node` and append them
                // to their parents
                while ws as i32 <= parents.last().unwrap().1 {
                    let (u, _ws) = parents.pop().unwrap();
                    parents.last_mut().unwrap().0.children.push(u);
                }

                // Push current node to the stack
                parents.push((node, ws as i32));
            }
        }

        // Append last nodes to their parents
        while parents.len() >= 2 {
            let (u, _ws) = parents.pop().unwrap();
            parents.last_mut().unwrap().0.children.push(u);
        }

        parents.pop().unwrap().0
    }
}

fn pick(tree: &Tree) {
    if tree.children.is_empty() {
        println!("Nothing to pick from! See `wtp --help` for more options.");
        return;
    }

    let mut t = tree;
    while !t.children.is_empty() {
        let options = t.children.iter().map(|n| &n.key).collect();
        let select = Select::new("", options)
            .with_vim_mode(true);
        let res = select.raw_prompt().unwrap();

        t = &t.children[res.index];
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let (flags, tree_id) = args();

    let dir = directories::BaseDirs::new().unwrap()
        .data_dir().join("WhatToPick");

    let file = dir.join(tree_id.unwrap_or("default".into()));

    // Print help message
    if flags.contains("--help") || flags.contains("-h") {
        println!("{}", HELP_MESSAGE);
    }
    // Create directory and open editor to edit the file
    else if flags.contains("--edit") || flags.contains("-e") {
        fs::create_dir_all(&dir)
            .expect(&format!("Unable to create directory <{}>", dir.to_string_lossy()));
        spawn_editor(file.as_path())?;
    }
    // Print the file path
    else if flags.contains("--file") || flags.contains("-f") {
        println!("{}", file.to_string_lossy());
    }
    // Interactively decide what to pick
    else {
        let tree = Tree::from_file(file.as_path());
        pick(&tree);
    }

    Ok(())
}

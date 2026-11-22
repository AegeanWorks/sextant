//! Hand-rolled argument parsing: no external crates, deterministic errors.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    Help,
    Version,
    Inventory,
    Doctor { json: bool },
    Fmt { machine: String, out: Option<String>, check: bool },
    Diff { left: String, right: String, json: bool },
    Untrace { trace: String, out: Option<String> },
    Slice { machine: String, events: String, out: Option<String> },
    Explain { machine: String, prefix: String, event: String, out: Option<String> },
    Reachable { machine: String, depth: usize, out: Option<String> },
    Check { machine: String, json: bool },
    Replay { machine: String, events: String, out: Option<String> },
    Golden { machine: String, events: String, golden: String, update: bool },
    Explore { machine: String, depth: usize, json: bool },
    Equiv { left: String, right: String, depth: usize, out: Option<String> },
    Fuzz { machine: String, seeds: usize, count: usize, invariant: Option<String>, out: Option<String>, json: bool },
    Coverage { machine: String, events: String, out: Option<String> },
    Minimize { machine: String, events: String, invariant: Option<String>, out: Option<String> },
    Graph { machine: String, format: String, out: Option<String> },
    Generate { machine: String, seed: u64, count: usize, out: Option<String> },
    Demo { seed: u64 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CliError {
    pub message: String,
}

pub fn usage_text() -> String {
    [
        "sextant 0.1.0 - deterministic finite-state-machine tooling",
        "",
        "USAGE",
        "  sextant check <machine.sex> [--json]",
        "  sextant replay <machine.sex> <events.sex> [--out <trace.txt>]",
        "  sextant golden <machine.sex> <events.sex> <golden.txt> [--update]",
        "  sextant explore <machine.sex> [--depth N] [--json]",
        "  sextant equiv <left.sex> <right.sex> [--depth N] [--out <report.txt>]",
        "  sextant fuzz <machine.sex> [--seeds N] [--count M] [--invariant NAME] [--json] [--out <report.txt>]",
        "  sextant coverage <machine.sex> <events.sex> [--out <report.txt>]",
        "  sextant minimize <machine.sex> <events.sex> [--invariant NAME] [--out <events.sex>]",
        "  sextant graph <machine.sex> [--format dot|mermaid] [--out <file>]",
        "  sextant generate <machine.sex> --seed N [--count M] [--out <events.sex>]",
        "  sextant demo --scripted [--seed N]",
        "  sextant inventory",
        "  sextant doctor [--json]",
        "  sextant fmt <machine.sex> [--check] [--out <file>]",
        "  sextant diff <left.sex> <right.sex> [--json]",
        "  sextant untrace <trace.txt> [--out <events.sex>]",
        "  sextant slice <machine.sex> <events.sex> [--out <machine.sex>]",
        "  sextant explain <machine.sex> <prefix.events> <event> [--out <report.txt>]",
        "  sextant reachable <machine.sex> [--depth N] [--out <map.txt>]",
        "  sextant --help | --version",
        "",
        "EXIT CODES",
        "  0  success        1  verification or behaviour failure        2  usage error",
    ]
    .join("\n")
}

fn split(
    args: &[String],
    value_flags: &[&str],
    switch_flags: &[&str],
) -> Result<(Vec<String>, Vec<(String, String)>, Vec<String>), CliError> {
    let mut positional = Vec::new();
    let mut values = Vec::new();
    let mut switches = Vec::new();
    let mut index = 1usize;
    while index < args.len() {
        let token = &args[index];
        if value_flags.contains(&token.as_str()) {
            if index + 1 >= args.len() {
                return Err(CliError {
                    message: format!("{} needs a value", token),
                });
            }
            values.push((token.clone(), args[index + 1].clone()));
            index += 2;
        } else if switch_flags.contains(&token.as_str()) {
            switches.push(token.clone());
            index += 1;
        } else if token.starts_with('-') {
            return Err(CliError {
                message: format!("unknown option '{}'", token),
            });
        } else {
            positional.push(token.clone());
            index += 1;
        }
    }
    Ok((positional, values, switches))
}

fn exactly_one(positional: &[String], label: &str) -> Result<String, CliError> {
    if positional.len() != 1 {
        return Err(CliError {
            message: format!("expected exactly one {} argument", label),
        });
    }
    Ok(positional[0].clone())
}

fn flag_value<'a>(
    values: &'a [(String, String)],
    flag: &str,
) -> Result<Option<&'a str>, CliError> {
    let mut found = None;
    for (name, value) in values {
        if name == flag {
            if found.is_some() {
                return Err(CliError {
                    message: format!("{} may be given only once", flag),
                });
            }
            found = Some(value.as_str());
        }
    }
    Ok(found)
}

fn usize_value(values: &[(String, String)], flag: &str, default: usize) -> Result<usize, CliError> {
    match flag_value(values, flag)? {
        None => Ok(default),
        Some(raw) => raw.parse::<usize>().map_err(|_| CliError {
            message: format!("{} must be a non-negative integer, got '{}'", flag, raw),
        }),
    }
}

fn u64_required(values: &[(String, String)], flag: &str) -> Result<u64, CliError> {
    match flag_value(values, flag)? {
        None => Err(CliError {
            message: format!("{} is required", flag),
        }),
        Some(raw) => raw.parse::<u64>().map_err(|_| CliError {
            message: format!("{} must be a non-negative integer, got '{}'", flag, raw),
        }),
    }
}

fn u64_optional(values: &[(String, String)], flag: &str, default: u64) -> Result<u64, CliError> {
    match flag_value(values, flag)? {
        None => Ok(default),
        Some(raw) => raw.parse::<u64>().map_err(|_| CliError {
            message: format!("{} must be a non-negative integer, got '{}'", flag, raw),
        }),
    }
}

pub fn parse_args(args: &[String]) -> Result<Command, CliError> {
    if args.is_empty() {
        return Ok(Command::Help);
    }
    match args[0].as_str() {
        "--help" | "-h" | "help" => Ok(Command::Help),
        "--version" | "-V" => Ok(Command::Version),
        "explain" => {
            let (positional, values, _) = split(args, &["--out"], &[])?;
            if positional.len() != 3 {
                return Err(CliError {
                    message: "explain needs a machine file, a prefix script and an event name".to_string(),
                });
            }
            Ok(Command::Explain {
                machine: positional[0].clone(),
                prefix: positional[1].clone(),
                event: positional[2].clone(),
                out: flag_value(&values, "--out")?.map(str::to_string),
            })
        }
        "reachable" => {
            let (positional, values, _) = split(args, &["--depth", "--out"], &[])?;
            Ok(Command::Reachable {
                machine: exactly_one(&positional, "machine")?,
                depth: usize_value(&values, "--depth", 8)?,
                out: flag_value(&values, "--out")?.map(str::to_string),
            })
        }
        "untrace" => {
            let (positional, values, _) = split(args, &["--out"], &[])?;
            Ok(Command::Untrace {
                trace: exactly_one(&positional, "trace")?,
                out: flag_value(&values, "--out")?.map(str::to_string),
            })
        }
        "slice" => {
            let (positional, values, _) = split(args, &["--out"], &[])?;
            if positional.len() != 2 {
                return Err(CliError {
                    message: "slice needs a machine file and an event script".to_string(),
                });
            }
            Ok(Command::Slice {
                machine: positional[0].clone(),
                events: positional[1].clone(),
                out: flag_value(&values, "--out")?.map(str::to_string),
            })
        }
        "fmt" => {
            let (positional, values, switches) = split(args, &["--out"], &["--check"])?;
            Ok(Command::Fmt {
                machine: exactly_one(&positional, "machine")?,
                out: flag_value(&values, "--out")?.map(str::to_string),
                check: switches.iter().any(|flag| flag == "--check"),
            })
        }
        "diff" => {
            let (positional, _, switches) = split(args, &[], &["--json"])?;
            if positional.len() != 2 {
                return Err(CliError {
                    message: "diff needs two machine files".to_string(),
                });
            }
            Ok(Command::Diff {
                left: positional[0].clone(),
                right: positional[1].clone(),
                json: switches.iter().any(|flag| flag == "--json"),
            })
        }
        "doctor" => {
            let (positional, _, switches) = split(args, &[], &["--json"])?;
            if !positional.is_empty() {
                return Err(CliError {
                    message: "doctor takes no positional arguments".to_string(),
                });
            }
            Ok(Command::Doctor {
                json: switches.iter().any(|flag| flag == "--json"),
            })
        }
        "inventory" => {
            let (positional, _, _) = split(args, &[], &[])?;
            if !positional.is_empty() {
                return Err(CliError {
                    message: "inventory takes no arguments".to_string(),
                });
            }
            Ok(Command::Inventory)
        }
        "check" => {
            let (positional, _, switches) = split(args, &[], &["--json"])?;
            Ok(Command::Check {
                machine: exactly_one(&positional, "machine")?,
                json: switches.iter().any(|flag| flag == "--json"),
            })
        }
        "replay" => {
            let (positional, values, _) = split(args, &["--out"], &[])?;
            if positional.len() != 2 {
                return Err(CliError {
                    message: "replay needs a machine file and an event script".to_string(),
                });
            }
            Ok(Command::Replay {
                machine: positional[0].clone(),
                events: positional[1].clone(),
                out: flag_value(&values, "--out")?.map(str::to_string),
            })
        }
        "golden" => {
            let (positional, _, switches) = split(args, &[], &["--update"])?;
            if positional.len() != 3 {
                return Err(CliError {
                    message: "golden needs a machine file, an event script and a golden file".to_string(),
                });
            }
            let update = switches.iter().any(|flag| flag == "--update");
            Ok(Command::Golden {
                machine: positional[0].clone(),
                events: positional[1].clone(),
                golden: positional[2].clone(),
                update,
            })
        }
        "explore" => {
            let (positional, values, switches) = split(args, &["--depth"], &["--json"])?;
            Ok(Command::Explore {
                machine: exactly_one(&positional, "machine")?,
                depth: usize_value(&values, "--depth", 8)?,
                json: switches.iter().any(|flag| flag == "--json"),
            })
        }
        "fuzz" => {
            let (positional, values, switches) = split(
                args,
                &["--seeds", "--count", "--invariant", "--out"],
                &["--json"],
            )?;
            Ok(Command::Fuzz {
                machine: exactly_one(&positional, "machine")?,
                seeds: usize_value(&values, "--seeds", 64)?,
                count: usize_value(&values, "--count", 12)?,
                invariant: flag_value(&values, "--invariant")?.map(str::to_string),
                out: flag_value(&values, "--out")?.map(str::to_string),
                json: switches.iter().any(|flag| flag == "--json"),
            })
        }
        "equiv" => {
            let (positional, values, _) = split(args, &["--depth", "--out"], &[])?;
            if positional.len() != 2 {
                return Err(CliError {
                    message: "equiv needs two machine files".to_string(),
                });
            }
            Ok(Command::Equiv {
                left: positional[0].clone(),
                right: positional[1].clone(),
                depth: usize_value(&values, "--depth", 4)?,
                out: flag_value(&values, "--out")?.map(str::to_string),
            })
        }
        "coverage" => {
            let (positional, values, _) = split(args, &["--out"], &[])?;
            if positional.len() != 2 {
                return Err(CliError {
                    message: "coverage needs a machine file and an event script".to_string(),
                });
            }
            Ok(Command::Coverage {
                machine: positional[0].clone(),
                events: positional[1].clone(),
                out: flag_value(&values, "--out")?.map(str::to_string),
            })
        }
        "minimize" => {
            let (positional, values, _) = split(args, &["--invariant", "--out"], &[])?;
            if positional.len() != 2 {
                return Err(CliError {
                    message: "minimize needs a machine file and a failing event script".to_string(),
                });
            }
            Ok(Command::Minimize {
                machine: positional[0].clone(),
                events: positional[1].clone(),
                invariant: flag_value(&values, "--invariant")?.map(str::to_string),
                out: flag_value(&values, "--out")?.map(str::to_string),
            })
        }
        "graph" => {
            let (positional, values, _) = split(args, &["--format", "--out"], &[])?;
            let format = flag_value(&values, "--format")?.unwrap_or("dot").to_string();
            if crate::core::parse_graph_format(&format).is_none() {
                return Err(CliError {
                    message: format!("unknown graph format '{}' (use dot or mermaid)", format),
                });
            }
            Ok(Command::Graph {
                machine: exactly_one(&positional, "machine")?,
                format,
                out: flag_value(&values, "--out")?.map(str::to_string),
            })
        }
        "generate" => {
            let (positional, values, _) = split(args, &["--seed", "--count", "--out"], &[])?;
            Ok(Command::Generate {
                machine: exactly_one(&positional, "machine")?,
                seed: u64_required(&values, "--seed")?,
                count: usize_value(&values, "--count", 20)?,
                out: flag_value(&values, "--out")?.map(str::to_string),
            })
        }
        "demo" => {
            let (positional, values, switches) = split(args, &["--seed"], &["--scripted"])?;
            if !positional.is_empty() {
                return Err(CliError {
                    message: "demo takes no positional arguments".to_string(),
                });
            }
            if !switches.iter().any(|flag| flag == "--scripted") {
                return Err(CliError {
                    message: "demo requires --scripted".to_string(),
                });
            }
            Ok(Command::Demo {
                seed: u64_optional(&values, "--seed", 0)?,
            })
        }
        other => Err(CliError {
            message: format!("unknown command '{}'", other),
        }),
    }
}

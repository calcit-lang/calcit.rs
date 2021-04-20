#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate nanoid;

mod builtins;
mod call_stack;
mod data;
mod primes;
mod program;
mod runner;
mod snapshot;

use call_stack::StackKind;
use cirru_edn::parse_cirru_edn;
use dirs::home_dir;
use primes::CalcitData::*;
use std::fs;
use std::path::Path;

fn main() -> Result<(), String> {
  let cli_matches = clap::App::new("Calcit Runner")
    .version(primes::CALCI_VERSION)
    .author("Jon. <jiyinyiyong@gmail.com>")
    .about("Calcit Runner")
    .arg(
      clap::Arg::with_name("once")
        .help("disable watching mode")
        .default_value("false")
        .short("1")
        .long("once")
        .takes_value(false),
    )
    .arg(
      clap::Arg::with_name("input")
        .help("entry file path, defaults to compact.cirru")
        .default_value("compact.cirru")
        .index(1),
    )
    .get_matches();

  let eval_once = cli_matches.is_present("once");
  println!("once: {}", eval_once);

  // load core libs
  let bytes = include_bytes!("./cirru/calcit-core.cirru");
  let core_content = String::from_utf8_lossy(bytes).to_string();
  let core_data = parse_cirru_edn(core_content)?;
  let core_snapshot = snapshot::load_snapshot_data(core_data)?;

  // load entry file
  let entry_path = Path::new(cli_matches.value_of("input").unwrap());
  let content = fs::read_to_string(entry_path).expect("expected a Cirru snapshot");
  let data = parse_cirru_edn(content)?;
  // println!("reading: {}", content);
  let mut snapshot = snapshot::load_snapshot_data(data)?;

  // attach modules
  for module_path in &snapshot.configs.modules {
    let module_data = load_module(&module_path, entry_path.parent().unwrap())?;
    for (k, v) in &module_data.files {
      snapshot.files.insert(k.clone(), v.clone());
    }
  }
  for (k, v) in core_snapshot.files {
    snapshot.files.insert(k.clone(), v.clone());
  }

  // println!("{:?}", s);

  // println!("code: {:?}", );
  let program_code = program::extract_program_data(&snapshot)?;

  // println!("{:?}", program::lookup_evaled_def("a", "b"));
  // TODO simulate program state
  // program::write_evaled_def("a", "b", CalcitBool(true))?;
  // println!("{:?}", program::lookup_evaled_def("a", "b"));

  let (init_ns, init_def) = extract_ns_def(&snapshot.configs.init_fn)?;
  match program::lookup_ns_def(&init_ns, &init_def, &program_code) {
    None => Err(String::from("Invalid entry")),
    Some(expr) => {
      call_stack::push_call_stack(
        &init_ns,
        &init_def,
        StackKind::Fn,
        &None,
        &im::Vector::new(),
      );
      let entry = runner::evaluate_expr(&expr, &im::HashMap::new(), &init_ns, &program_code)?;
      match entry {
        CalcitFn(_, f_ns, _, def_scope, args, body) => {
          let result = runner::run_fn(
            im::Vector::new(),
            &def_scope,
            &args,
            &body,
            &f_ns,
            &program_code,
          );
          match result {
            Ok(v) => {
              println!("result: {}", v);
            }
            Err(falure) => {
              println!("failed, {}", falure);
              call_stack::display_stack(&falure);
            }
          }
          Ok(())
        }
        _ => Err(format!("expected function entry, got: {}", entry)),
      }
    }
  }
}

fn extract_ns_def(s: &str) -> Result<(String, String), String> {
  let pieces: Vec<&str> = (&s).split('/').collect();
  if pieces.len() == 2 {
    Ok((pieces[0].to_string(), pieces[1].to_string()))
  } else {
    Err(format!("invalid ns format: {}", s))
  }
}

fn load_module(path: &str, base_dir: &Path) -> Result<snapshot::Snapshot, String> {
  let mut file_path = String::from(path);
  if file_path.ends_with('/') {
    file_path.push_str("compact.cirru");
  }

  let fullpath: String = if file_path.starts_with("./") {
    let new_path = base_dir.join(file_path);
    new_path.to_str().unwrap().to_string()
  } else if file_path.starts_with('/') {
    file_path
  } else {
    match home_dir() {
      Some(buf) => {
        let home = buf.as_path();
        let p = home.join(".config/calcit/modules/").join(file_path);
        p.to_str().unwrap().to_string()
      }
      None => return Err(String::from("failed to load $HOME")),
    }
  };

  println!("loading module: {}", fullpath);

  let content = fs::read_to_string(&fullpath).expect("expected a Cirru snapshot");
  let data = parse_cirru_edn(content)?;
  // println!("reading: {}", content);
  let snapshot = snapshot::load_snapshot_data(data)?;
  Ok(snapshot)
}

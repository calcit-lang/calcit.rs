pub mod preprocess;

use crate::builtins;
use crate::builtins::{is_proc_name, is_syntax_name};
use crate::call_stack;
use crate::call_stack::{push_call_stack, StackKind};
use crate::primes::Calcit;
use crate::primes::{CalcitItems, CalcitScope, CrListWrap, SymbolResolved::*, CORE_NS};
use crate::program;

pub fn evaluate_expr(
  expr: &Calcit,
  scope: &CalcitScope,
  file_ns: &str,
  program_code: &program::ProgramCodeData,
) -> Result<Calcit, String> {
  // println!("eval code: {}", expr.lisp_str());

  match expr {
    Calcit::Nil => Ok(expr.to_owned()),
    Calcit::Bool(_) => Ok(expr.to_owned()),
    Calcit::Number(_) => Ok(expr.to_owned()),
    Calcit::Symbol(s, ..) if s == "&" => Ok(expr.to_owned()),
    Calcit::Symbol(s, ns, _at_def, resolved) => match resolved {
      Some(ResolvedDef(r_ns, r_def, _import_rule)) => {
        let v = evaluate_symbol(r_def, scope, r_ns, program_code)?;
        match v {
          Calcit::Thunk(_code, Some(data)) => Ok(*data),
          // extra check to make sure code in thunks evaluated
          Calcit::Thunk(code, None) => {
            let evaled_v = evaluate_expr(&code, scope, file_ns, program_code)?;
            // and write back to program state to fix duplicated evalution
            // still using thunk since js and IR requires bare code
            program::write_evaled_def(r_ns, r_def, Calcit::Thunk(code, Some(Box::new(evaled_v.to_owned()))))?;
            Ok(evaled_v)
          }
          _ => Ok(v),
        }
      }
      _ => evaluate_symbol(&s, scope, &ns, program_code),
    },
    Calcit::Keyword(_) => Ok(expr.to_owned()),
    Calcit::Str(_) => Ok(expr.to_owned()),
    Calcit::Thunk(code, v) => match v {
      None => evaluate_expr(code, scope, file_ns, program_code),
      Some(data) => Ok(*data.to_owned()),
    },
    Calcit::Ref(_) => Ok(expr.to_owned()),
    Calcit::Tuple(..) => Ok(expr.to_owned()),
    Calcit::Recur(_) => unreachable!("recur not expected to be from symbol"),
    Calcit::List(xs) => match xs.get(0) {
      None => Err(format!("cannot evaluate empty expr: {}", expr)),
      Some(x) => {
        // println!("eval expr: {}", expr.lisp_str());
        // println!("eval expr: {}", x);

        let mut added_stack = false;

        let v = evaluate_expr(&x, scope, file_ns, program_code)?;
        let rest_nodes = xs.skip(1);
        let ret = match &v {
          Calcit::Proc(p) => {
            let values = evaluate_args(&rest_nodes, scope, file_ns, program_code)?;
            push_call_stack(file_ns, &p, StackKind::Proc, Calcit::Nil, &values);
            added_stack = true;
            if p.starts_with('.') {
              builtins::meta::invoke_method(&p.strip_prefix('.').unwrap(), &values, program_code)
            } else {
              // println!("proc: {}", expr);
              builtins::handle_proc(&p, &values)
            }
          }
          Calcit::Syntax(s, def_ns) => {
            push_call_stack(file_ns, &s, StackKind::Syntax, expr.to_owned(), &rest_nodes);
            added_stack = true;
            builtins::handle_syntax(&s, &rest_nodes, scope, def_ns, program_code)
          }
          Calcit::Fn(name, def_ns, _, def_scope, args, body) => {
            let values = evaluate_args(&rest_nodes, scope, file_ns, program_code)?;
            push_call_stack(file_ns, &name, StackKind::Fn, expr.to_owned(), &values);
            added_stack = true;
            run_fn(&values, &def_scope, args, body, def_ns, program_code)
          }
          Calcit::Macro(name, def_ns, _, args, body) => {
            println!(
              "[Warn] macro should already be handled during preprocessing: {}",
              Calcit::List(xs.to_owned()).lisp_str()
            );

            // TODO moving to preprocess
            let mut current_values = rest_nodes.to_owned();
            // println!("eval macro: {} {}", x, expr.lisp_str()));
            // println!("macro... {} {}", x, CrListWrap(current_values.to_owned()));

            push_call_stack(file_ns, &name, StackKind::Macro, expr.to_owned(), &rest_nodes);
            added_stack = true;

            Ok(loop {
              // need to handle recursion
              let body_scope = bind_args(&args, &current_values, &im::HashMap::new())?;
              let code = evaluate_lines(&body, &body_scope, def_ns, program_code)?;
              match code {
                Calcit::Recur(ys) => {
                  current_values = ys;
                }
                _ => {
                  // println!("gen code: {} {}", x, &code.lisp_str()));
                  break evaluate_expr(&code, scope, file_ns, program_code)?;
                }
              }
            })
          }
          Calcit::Keyword(k) => {
            if rest_nodes.len() == 1 {
              let v = evaluate_expr(&rest_nodes[0], scope, file_ns, program_code)?;

              if let Calcit::Map(m) = v {
                match m.get(&Calcit::Keyword(k.to_owned())) {
                  Some(value) => Ok(value.to_owned()),
                  None => Ok(Calcit::Nil),
                }
              } else {
                Err(format!("expected a hashmap, got {}", v))
              }
            } else {
              Err(format!(
                "keyword only takes 1 argument, got: {}",
                CrListWrap(rest_nodes)
              ))
            }
          }
          Calcit::Symbol(s, ns, at_def, resolved) => Err(format!(
            "cannot evaluate symbol directly: {}/{} in {}, {:?}",
            ns, s, at_def, resolved
          )),
          a => Err(format!(
            "cannot be used as operator: {} in {}",
            a,
            CrListWrap(xs.to_owned())
          )),
        };

        if added_stack && ret.is_ok() {
          call_stack::pop_call_stack();
        }

        ret
      }
    },
    Calcit::Set(_) => Err(String::from("unexpected set for expr")),
    Calcit::Map(_) => Err(String::from("unexpected map for expr")),
    Calcit::Record(..) => Err(String::from("unexpected record for expr")),
    Calcit::Proc(_) => Ok(expr.to_owned()),
    Calcit::Macro(..) => Ok(expr.to_owned()),
    Calcit::Fn(..) => Ok(expr.to_owned()),
    Calcit::Syntax(_, _) => Ok(expr.to_owned()),
  }
}

pub fn evaluate_symbol(
  sym: &str,
  scope: &CalcitScope,
  file_ns: &str,
  program_code: &program::ProgramCodeData,
) -> Result<Calcit, String> {
  match parse_ns_def(&sym) {
    Some((ns_part, def_part)) => match program::lookup_ns_target_in_import(file_ns, &ns_part, program_code) {
      Some(target_ns) => match eval_symbol_from_program(&def_part, &target_ns, program_code) {
        Ok(v) => Ok(v),
        Err(e) => Err(e),
      },
      None => Err(format!("unknown ns target: {}/{}", ns_part, def_part)),
    },
    None => {
      if is_syntax_name(sym) {
        return Ok(Calcit::Syntax(sym.to_owned(), file_ns.to_owned()));
      }
      if scope.contains_key(sym) {
        // although scope is detected first, it would trigger warning during preprocess
        return Ok(scope.get(sym).unwrap().to_owned());
      }
      if is_proc_name(sym) {
        return Ok(Calcit::Proc(sym.to_owned()));
      }
      if program::lookup_def_code(CORE_NS, sym, program_code).is_some() {
        return eval_symbol_from_program(sym, CORE_NS, program_code);
      }
      if program::has_def_code(file_ns, sym, program_code) {
        return eval_symbol_from_program(sym, file_ns, program_code);
      }
      match program::lookup_def_target_in_import(file_ns, sym, program_code) {
        Some(target_ns) => eval_symbol_from_program(sym, &target_ns, program_code),
        None => {
          let vars: Vec<&String> = scope.keys().collect();
          Err(format!("unknown symbol `{}` in {:?}", sym, vars))
        }
      }
    }
  }
}

pub fn parse_ns_def(s: &str) -> Option<(String, String)> {
  let pieces: Vec<&str> = s.split('/').collect();
  if pieces.len() == 2 {
    if !pieces[0].is_empty() && !pieces[1].is_empty() {
      Some((pieces[0].to_owned(), pieces[1].to_owned()))
    } else {
      None
    }
  } else {
    None
  }
}

fn eval_symbol_from_program(sym: &str, ns: &str, program_code: &program::ProgramCodeData) -> Result<Calcit, String> {
  match program::lookup_evaled_def(ns, sym) {
    Some(v) => Ok(v),
    None => match program::lookup_def_code(ns, sym, program_code) {
      Some(code) => {
        let v = evaluate_expr(&code, &im::HashMap::new(), ns, program_code)?;
        program::write_evaled_def(ns, sym, v.to_owned())?;
        Ok(v)
      }
      None => Err(format!("cannot find code for def: {}/{}", ns, sym)),
    },
  }
}

pub fn run_fn(
  values: &CalcitItems,
  scope: &CalcitScope,
  args: &CalcitItems,
  body: &CalcitItems,
  file_ns: &str,
  program_code: &program::ProgramCodeData,
) -> Result<Calcit, String> {
  let mut current_values = values.to_owned();
  loop {
    let body_scope = bind_args(args, &current_values, scope)?;
    let v = evaluate_lines(body, &body_scope, file_ns, program_code)?;
    match v {
      Calcit::Recur(xs) => {
        current_values = xs;
      }
      result => return Ok(result),
    }
  }
}

/// create new scope by wrting new args
/// notice that `&` is a mark for spreading, `?` for optional arguments
pub fn bind_args(args: &CalcitItems, values: &CalcitItems, base_scope: &CalcitScope) -> Result<CalcitScope, String> {
  // TODO arguments spreading syntax
  // if values.len() != args.len() {
  //   return Err(format!(
  //     "arguments length mismatch: {} ... {}",
  //     Calcit::List(values.to_owned()),
  //     Calcit::List(args.to_owned()),
  //   ));
  // }
  let mut scope = base_scope.to_owned();
  let mut spreading = false;
  let mut optional = false;
  let mut collected_args = args.to_owned();
  let mut collected_values = values.to_owned();
  while let Some(a) = collected_args.pop_front() {
    if spreading {
      match a {
        Calcit::Symbol(s, ..) if s == "&" => return Err(format!("invalid & in args: {:?}", args)),
        Calcit::Symbol(s, ..) if s == "?" => return Err(format!("invalid ? in args: {:?}", args)),
        Calcit::Symbol(s, ..) => {
          let mut chunk: CalcitItems = im::vector![];
          while let Some(v) = collected_values.pop_front() {
            chunk.push_back(v);
          }
          scope.insert(s, Calcit::List(chunk));
          if !collected_args.is_empty() {
            return Err(format!(
              "extra args `{}` after spreading in `{}`",
              CrListWrap(collected_args),
              CrListWrap(args.to_owned()),
            ));
          }
        }
        b => return Err(format!("invalid argument name: {}", b)),
      }
    } else {
      match a {
        Calcit::Symbol(s, ..) if s == "&" => spreading = true,
        Calcit::Symbol(s, ..) if s == "?" => optional = true,
        Calcit::Symbol(s, ..) => match collected_values.pop_front() {
          Some(v) => {
            scope.insert(s.to_owned(), v.to_owned());
          }
          None => {
            if optional {
              scope.insert(s.to_owned(), Calcit::Nil);
            } else {
              return Err(format!(
                "too few values `{}` passed to args `{}`",
                CrListWrap(values.to_owned()),
                CrListWrap(args.to_owned())
              ));
            }
          }
        },
        b => return Err(format!("invalid argument name: {}", b)),
      }
    }
  }
  if collected_values.is_empty() {
    Ok(scope)
  } else {
    Err(format!(
      "extra args `{}` not handled while passing values `{}` to args `{}`",
      CrListWrap(collected_values),
      CrListWrap(values.to_owned()),
      CrListWrap(args.to_owned()),
    ))
  }
}

pub fn evaluate_lines(
  lines: &CalcitItems,
  scope: &CalcitScope,
  file_ns: &str,
  program_code: &program::ProgramCodeData,
) -> Result<Calcit, String> {
  let mut ret: Calcit = Calcit::Nil;
  for line in lines {
    match evaluate_expr(line, scope, file_ns, program_code) {
      Ok(v) => ret = v,
      Err(e) => return Err(e),
    }
  }
  Ok(ret)
}

/// evaluate symbols before calling a function
/// notice that `&` is used to spread a list
pub fn evaluate_args(
  items: &CalcitItems,
  scope: &CalcitScope,
  file_ns: &str,
  program_code: &program::ProgramCodeData,
) -> Result<CalcitItems, String> {
  let mut ret: CalcitItems = im::vector![];
  let mut spreading = false;
  for item in items {
    match item {
      Calcit::Symbol(s, ..) if s == "&" => {
        spreading = true;
      }
      _ => match &evaluate_expr(item, scope, file_ns, program_code) {
        Ok(v) => {
          if spreading {
            match v {
              Calcit::List(xs) => {
                for x in xs {
                  // extract thunk before calling functions
                  let y = match x {
                    Calcit::Thunk(code, v) => match v {
                      None => evaluate_expr(code, scope, file_ns, program_code)?,
                      Some(data) => *data.to_owned(),
                    },
                    _ => x.to_owned(),
                  };
                  ret.push_back(y.to_owned());
                }
                spreading = false
              }
              a => return Err(format!("expected list for spreading, got: {}", a)),
            }
          } else {
            // extract thunk before calling functions
            let y = match v {
              Calcit::Thunk(code, value) => match value {
                None => evaluate_expr(code, scope, file_ns, program_code)?,
                Some(data) => *data.to_owned(),
              },
              _ => v.to_owned(),
            };
            ret.push_back(y)
          }
        }
        Err(e) => return Err(e.to_owned()),
      },
    }
  }
  Ok(ret)
}

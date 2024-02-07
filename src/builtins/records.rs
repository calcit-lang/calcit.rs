use std::cmp::Ordering;
use std::ops::Rem;
use std::sync::Arc;

use cirru_edn::EdnTag;

use crate::calcit::{Calcit, CalcitCompactList, CalcitErr, CalcitRecord};

pub fn new_record(xs: &CalcitCompactList) -> Result<Calcit, CalcitErr> {
  if xs.is_empty() {
    return CalcitErr::err_nodes("new-record expected arguments, got:", xs);
  }
  let name_id: EdnTag = match &xs[0] {
    Calcit::Symbol { sym, .. } => EdnTag::new(sym),
    Calcit::Tag(k) => k.to_owned(),
    a => return CalcitErr::err_str(format!("new-record expected a name, got: {a}")),
  };

  let mut fields: Vec<EdnTag> = Vec::with_capacity(xs.len());
  let mut values: Vec<Calcit> = Vec::with_capacity(xs.len());

  for x in xs.into_iter().skip(1) {
    match x {
      Calcit::Symbol { sym, .. } | Calcit::Str(sym) => {
        fields.push(EdnTag::new(sym));
      }
      Calcit::Tag(s) => {
        fields.push(s.to_owned());
      }
      a => return CalcitErr::err_str(format!("new-record fields accepets tag/string, got a {a}")),
    }
    values.push(Calcit::Nil);
  }
  fields.sort_unstable(); // all values are nil

  // warn about dup
  let mut prev: EdnTag = EdnTag::new(""); // actually a invalid default...
  for (idx, x) in fields.iter().enumerate() {
    if idx > 0 {
      if x == &prev {
        return CalcitErr::err_str(format!("duplicated field for record: {}", Calcit::Tag(x.to_owned())));
      } else {
        prev = x.to_owned();
        // checked ok
      }
    } else {
      prev = x.to_owned()
    }
  }
  Ok(Calcit::Record(CalcitRecord {
    name: name_id,
    fields: Arc::new(fields),
    values: Arc::new(values),
    class: Arc::new(Calcit::Nil),
  }))
}

pub fn new_class_record(xs: &CalcitCompactList) -> Result<Calcit, CalcitErr> {
  if xs.is_empty() {
    return CalcitErr::err_nodes("new-record expected arguments, got:", xs);
  }
  let class: Calcit = match &xs[0] {
    a @ Calcit::Record { .. } => a.to_owned(),
    b => return CalcitErr::err_str(format!("new-class-record expected a class, got: {b}")),
  };
  let name_id: EdnTag = match &xs[1] {
    Calcit::Symbol { sym, .. } => EdnTag::new(sym),
    Calcit::Tag(k) => k.to_owned(),
    a => return CalcitErr::err_str(format!("new-record expected a name, got: {a}")),
  };

  let mut fields: Vec<EdnTag> = Vec::with_capacity(xs.len());
  let mut values: Vec<Calcit> = Vec::with_capacity(xs.len());

  for x in xs.into_iter().skip(2) {
    match x {
      Calcit::Symbol { sym, .. } | Calcit::Str(sym) => {
        fields.push(EdnTag::new(sym));
      }
      Calcit::Tag(s) => {
        fields.push(s.to_owned());
      }
      a => return CalcitErr::err_str(format!("new-record fields accepets tag/string, got a {a}")),
    }
    values.push(Calcit::Nil);
  }
  fields.sort_unstable(); // all values are nil

  // warn about dup
  let mut prev: EdnTag = EdnTag::new(""); // actually a invalid default...
  for (idx, x) in fields.iter().enumerate() {
    if idx > 0 {
      if x == &prev {
        return CalcitErr::err_str(format!("duplicated field for record: {}", Calcit::Tag(x.to_owned())));
      } else {
        prev = x.to_owned();
        // checked ok
      }
    } else {
      prev = x.to_owned()
    }
  }
  Ok(Calcit::Record(CalcitRecord {
    name: name_id,
    fields: Arc::new(fields),
    values: Arc::new(values),
    class: Arc::new(class),
  }))
}

pub fn call_record(xs: &CalcitCompactList) -> Result<Calcit, CalcitErr> {
  let args_size = xs.len();
  if args_size < 2 {
    return CalcitErr::err_nodes("&%{{}} expected at least 2 arguments, got:", xs);
  }
  match &xs[0] {
    Calcit::Record(CalcitRecord {
      name,
      fields: def_fields,
      values: v0,
      class,
    }) => {
      if (args_size - 1).rem(2) == 0 {
        let size = (args_size - 1) / 2;
        if size != def_fields.len() {
          return CalcitErr::err_str(format!("unexpected size in &%{{}}, {size} .. {}", def_fields.len()));
        }
        let mut values: Vec<Calcit> = (**v0).to_owned();

        for idx in 0..size {
          let k_idx = idx * 2 + 1;
          let v_idx = k_idx + 1;
          match &xs[k_idx] {
            Calcit::Tag(s) => match find_in_fields(def_fields, s) {
              Some(pos) => {
                values[pos] = xs[v_idx].to_owned();
              }
              None => return CalcitErr::err_str(format!("unexpected field {s} for {def_fields:?}")),
            },
            Calcit::Symbol { sym: s, .. } | Calcit::Str(s) => match find_in_fields(def_fields, &EdnTag::new(s)) {
              Some(pos) => {
                values[pos] = xs[v_idx].to_owned();
              }
              None => return CalcitErr::err_str(format!("unexpected field {s} for {def_fields:?}")),
            },
            a => return CalcitErr::err_str(format!("expected field in string/tag, got: {a}")),
          }
        }

        Ok(Calcit::Record(CalcitRecord {
          name: name.to_owned(),
          fields: def_fields.to_owned(),
          values: Arc::new(values),
          class: class.to_owned(),
        }))
      } else {
        CalcitErr::err_nodes("&%{{}} expected pairs, got:", xs)
      }
    }
    a => CalcitErr::err_str(format!("&%{{}} expected a record as prototype, got: {a}")),
  }
}

pub fn get_class(xs: &CalcitCompactList) -> Result<Calcit, CalcitErr> {
  let args_size = xs.len();
  if args_size != 1 {
    return CalcitErr::err_nodes("&record:class expected 1 argument, got:", xs);
  }
  match &xs[0] {
    Calcit::Record(CalcitRecord { class, .. }) => Ok((**class).to_owned()),
    a => CalcitErr::err_str(format!("&record:class expected a record as prototype, got: {a}")),
  }
}

pub fn with_class(xs: &CalcitCompactList) -> Result<Calcit, CalcitErr> {
  let args_size = xs.len();
  if args_size < 2 {
    return CalcitErr::err_nodes("&record:with-class expected at least 2 arguments, got:", xs);
  }
  match (&xs[0], &xs[1]) {
    (
      Calcit::Record(CalcitRecord {
        name,
        fields: def_fields,
        values: v0,
        ..
      }),
      c @ Calcit::Record { .. },
    ) => Ok(Calcit::Record(CalcitRecord {
      name: name.to_owned(),
      fields: def_fields.to_owned(),
      values: v0.to_owned(),
      class: Arc::new(c.to_owned()),
    })),
    (Calcit::Record { .. }, b) => CalcitErr::err_str(format!("&record:with-class expected a record as class, got: {b}")),
    (a, _b) => CalcitErr::err_str(format!("&record:with-class expected a record, got: {a}")),
  }
}

pub fn record_from_map(xs: &CalcitCompactList) -> Result<Calcit, CalcitErr> {
  if xs.len() != 2 {
    return CalcitErr::err_nodes("&record:from-map expected 2 arguments, got:", xs);
  }
  match (&xs[0], &xs[1]) {
    (Calcit::Record(CalcitRecord { name, fields, class, .. }), Calcit::Map(ys)) => {
      let mut pairs: Vec<(EdnTag, Calcit)> = Vec::with_capacity(fields.len());
      for (k, v) in ys {
        match k {
          Calcit::Str(s) => {
            pairs.push((EdnTag::new(s), v.to_owned()));
          }
          Calcit::Tag(s) => {
            pairs.push((s.to_owned(), v.to_owned()));
          }
          a => return CalcitErr::err_str(format!("unknown field {a}")),
        }
      }
      if fields.len() != pairs.len() {
        return CalcitErr::err_str(format!("invalid fields {pairs:?} for record {fields:?}"));
      }
      pairs.sort_by(|(a, _), (b, _)| a.cmp(b));
      let mut values: Vec<Calcit> = Vec::with_capacity(fields.len());
      for idx in 0..fields.len() {
        let (k, v) = &pairs[idx];
        if &fields[idx] == k {
          values.push(v.to_owned());
        } else {
          return CalcitErr::err_str(format!("field mismatch: {k} {} in {fields:?} {pairs:?}", fields[idx]));
        }
      }
      Ok(Calcit::Record(CalcitRecord {
        name: name.to_owned(),
        fields: fields.to_owned(),
        values: Arc::new(values),
        class: class.to_owned(),
      }))
    }
    (a, b) => CalcitErr::err_str(format!("&record:from-map expected a record and a map, got: {a} {b}")),
  }
}

pub fn get_record_name(xs: &CalcitCompactList) -> Result<Calcit, CalcitErr> {
  if xs.len() != 1 {
    return CalcitErr::err_nodes("&record:get-name expected record, got::", xs);
  }
  match &xs[0] {
    Calcit::Record(CalcitRecord { name, .. }) => Ok(Calcit::Tag(name.to_owned())),
    a => CalcitErr::err_str(format!("&record:get-name expected record, got: {a}")),
  }
}
pub fn turn_map(xs: &CalcitCompactList) -> Result<Calcit, CalcitErr> {
  if xs.len() != 1 {
    return CalcitErr::err_nodes("&record:to-map expected 1 argument, got::", xs);
  }
  match &xs[0] {
    Calcit::Record(CalcitRecord { fields, values, .. }) => {
      let mut ys: rpds::HashTrieMapSync<Calcit, Calcit> = rpds::HashTrieMap::new_sync();
      for idx in 0..fields.len() {
        ys.insert_mut(Calcit::Tag(fields[idx].to_owned()), values[idx].to_owned());
      }
      Ok(Calcit::Map(ys))
    }
    a => CalcitErr::err_str(format!("&record:to-map expected a record, got: {a}")),
  }
}
pub fn matches(xs: &CalcitCompactList) -> Result<Calcit, CalcitErr> {
  if xs.len() != 2 {
    return CalcitErr::err_nodes("&record:matches? expected 2 arguments, got:", xs);
  }
  match (&xs[0], &xs[1]) {
    (
      Calcit::Record(CalcitRecord {
        name: left,
        fields: left_fields,
        ..
      }),
      Calcit::Record(CalcitRecord {
        name: right,
        fields: right_fields,
        ..
      }),
    ) => Ok(Calcit::Bool(left == right && left_fields == right_fields)),
    (a, b) => CalcitErr::err_str(format!("&record:matches? expected 2 records, got: {a} {b}")),
  }
}

/// returns position of target
pub fn find_in_fields(xs: &[EdnTag], y: &EdnTag) -> Option<usize> {
  if xs.is_empty() {
    return None;
  }
  let mut lower = 0;
  let mut upper = xs.len() - 1;

  while (upper - lower) > 1 {
    let pos = (lower + upper) >> 1;
    let v = xs[pos].to_owned();
    match y.cmp(&v) {
      Ordering::Less => upper = pos - 1,
      Ordering::Greater => lower = pos + 1,
      Ordering::Equal => return Some(pos),
    }
  }

  match y {
    _ if y == &xs[lower] => Some(lower),
    _ if y == &xs[upper] => Some(upper),
    _ => None,
  }
}

pub fn count(xs: &CalcitCompactList) -> Result<Calcit, CalcitErr> {
  if xs.len() != 1 {
    return CalcitErr::err_nodes("record count expected 1 argument::", xs);
  }
  match &xs[0] {
    Calcit::Record(CalcitRecord { fields, .. }) => Ok(Calcit::Number(fields.len() as f64)),
    a => CalcitErr::err_str(format!("record count expected a record, got: {a}")),
  }
}

pub fn contains_ques(xs: &CalcitCompactList) -> Result<Calcit, CalcitErr> {
  match (xs.get(0), xs.get(1)) {
    (Some(Calcit::Record(CalcitRecord { fields, .. })), Some(a)) => match a {
      Calcit::Str(k) | Calcit::Symbol { sym: k, .. } => Ok(Calcit::Bool(find_in_fields(fields, &EdnTag::new(k)).is_some())),
      Calcit::Tag(k) => Ok(Calcit::Bool(find_in_fields(fields, k).is_some())),
      a => CalcitErr::err_str(format!("contains? got invalid field for record: {a}")),
    },
    (Some(a), ..) => CalcitErr::err_str(format!("record contains? expected a record, got: {a}")),
    (None, ..) => CalcitErr::err_str(format!("record contains? expected 2 arguments, got: {xs:?}")),
  }
}

pub fn get(xs: &CalcitCompactList) -> Result<Calcit, CalcitErr> {
  match (xs.get(0), xs.get(1)) {
    (Some(Calcit::Record(CalcitRecord { fields, values, .. })), Some(a)) => match a {
      Calcit::Str(k) | Calcit::Symbol { sym: k, .. } => match find_in_fields(fields, &EdnTag::new(k)) {
        Some(idx) => Ok(values[idx].to_owned()),
        None => Ok(Calcit::Nil),
      },
      Calcit::Tag(k) => match find_in_fields(fields, k) {
        Some(idx) => Ok(values[idx].to_owned()),
        None => Ok(Calcit::Nil),
      },
      a => CalcitErr::err_str(format!("record field expected to be string/tag, got: {a}")),
    },
    (Some(a), ..) => CalcitErr::err_str(format!("record &get expected record, got: {a}")),
    (None, ..) => CalcitErr::err_str(format!("record &get expected 2 arguments, got: {xs:?}")),
  }
}

pub fn assoc(xs: &CalcitCompactList) -> Result<Calcit, CalcitErr> {
  match (xs.get(0), xs.get(1), xs.get(2)) {
    (
      Some(Calcit::Record(CalcitRecord {
        name,
        fields,
        values,
        class,
      })),
      Some(a),
      Some(b),
    ) => match a {
      Calcit::Str(s) | Calcit::Symbol { sym: s, .. } => match find_in_fields(fields, &EdnTag::new(s)) {
        Some(pos) => {
          let mut new_values = (**values).to_owned();
          new_values[pos] = b.to_owned();
          Ok(Calcit::Record(CalcitRecord {
            name: name.to_owned(),
            fields: fields.to_owned(),
            values: Arc::new(new_values),
            class: class.to_owned(),
          }))
        }
        None => CalcitErr::err_str(format!("invalid field `{s}` for {fields:?}")),
      },
      Calcit::Tag(s) => match find_in_fields(fields, s) {
        Some(pos) => {
          let mut new_values = (**values).to_owned();
          new_values[pos] = b.to_owned();
          Ok(Calcit::Record(CalcitRecord {
            name: name.to_owned(),
            fields: fields.to_owned(),
            values: Arc::new(new_values),
            class: class.to_owned(),
          }))
        }
        None => CalcitErr::err_str(format!("invalid field `{s}` for {fields:?}")),
      },
      a => CalcitErr::err_str(format!("invalid field `{a}` for {fields:?}")),
    },
    (Some(a), ..) => CalcitErr::err_str(format!("record:assoc expected a record, got: {a}")),
    (None, ..) => CalcitErr::err_nodes("record:assoc expected 3 arguments, got:", xs),
  }
}

pub fn extend_as(xs: &CalcitCompactList) -> Result<Calcit, CalcitErr> {
  if xs.len() != 4 {
    return CalcitErr::err_nodes("record:extend-as expected 4 arguments, got::", xs);
  }
  match (xs.get(0), xs.get(1), xs.get(2), xs.get(3)) {
    (Some(Calcit::Record(CalcitRecord { fields, values, class, .. })), Some(n), Some(a), Some(new_value)) => match a {
      Calcit::Str(s) | Calcit::Symbol { sym: s, .. } => match find_in_fields(fields, &EdnTag::new(s)) {
        Some(_pos) => CalcitErr::err_str(format!("field `{s}` already existed")),
        None => extend_record_field(&EdnTag::new(s), n, fields, values, new_value, class),
      },
      Calcit::Tag(s) => match find_in_fields(fields, s) {
        Some(_pos) => CalcitErr::err_str(format!("field `{s}` already existed")),
        None => extend_record_field(s, n, fields, values, new_value, class),
      },
      a => CalcitErr::err_str(format!("invalid field `{a}` for {fields:?}")),
    },
    (Some(a), ..) => CalcitErr::err_str(format!("record:extend-as expected a record, got: {a}")),
    (None, ..) => CalcitErr::err_nodes("record:extend-as expected 4 arguments, got:", xs),
  }
}

fn extend_record_field(
  idx_s: &EdnTag,
  n: &Calcit,
  fields: &[EdnTag],
  values: &[Calcit],
  new_value: &Calcit,
  class: &Calcit,
) -> Result<Calcit, CalcitErr> {
  let mut next_fields: Vec<EdnTag> = Vec::with_capacity(fields.len());
  let mut next_values: Vec<Calcit> = Vec::with_capacity(fields.len());
  let mut inserted: bool = false;

  for (i, k) in fields.iter().enumerate() {
    if inserted {
      next_fields.push(k.to_owned());
      next_values.push(values[i].to_owned());
    } else {
      match idx_s.cmp(k) {
        Ordering::Less => {
          next_fields.push(idx_s.to_owned());
          next_values.push(new_value.to_owned());

          next_fields.push(k.to_owned());
          next_values.push(values[i].to_owned());
          inserted = true;
        }
        Ordering::Greater => {
          next_fields.push(k.to_owned());
          next_values.push(values[i].to_owned());
        }
        Ordering::Equal => {
          unreachable!("does not equal")
        }
      }
    }
  }
  if !inserted {
    next_fields.push(idx_s.to_owned());
    next_values.push(new_value.to_owned());
  }

  let new_name_id: EdnTag = match n {
    Calcit::Str(s) | Calcit::Symbol { sym: s, .. } => EdnTag::new(s),
    Calcit::Tag(s) => s.to_owned(),
    _ => return CalcitErr::err_str("expected record name"),
  };

  Ok(Calcit::Record(CalcitRecord {
    name: new_name_id,
    fields: Arc::new(next_fields),
    values: Arc::new(next_values),
    class: Arc::new(class.to_owned()),
  }))
}

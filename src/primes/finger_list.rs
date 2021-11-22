use std::fmt;

use core::cmp::Ord;
use std::cmp::Eq;
use std::cmp::Ordering;
use std::cmp::Ordering::*;
use std::fmt::{Debug, Display};
use std::hash::{Hash, Hasher};

use fingertrees::measure::{Measured, Size};
use fingertrees::monoid::Sum;
use std::ops::Index;

use fingertrees::{ArcRefs, FingerTree};

#[derive(Debug, Clone)]
pub struct FingerList<T>(FingerTree<ArcRefs, Size<T>>)
where
  T: Clone;

impl<T> fmt::Display for FingerList<T>
where
  T: Debug + Clone + Hash + Ord + Display,
{
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("(&filter-list ")?;

    for x in self.into_iter() {
      f.write_str(" ")?;
      f.write_str(&x.to_string())?;
    }

    f.write_str(")")
  }
}

impl<T> Hash for FingerList<T>
where
  T: Debug + Clone + Hash + Ord + Display,
{
  fn hash<H>(&self, _state: &mut H)
  where
    H: Hasher,
  {
    for item in self.iter() {
      item.hash(_state);
    }
  }
}

impl<T> Ord for FingerList<T>
where
  T: Debug + Clone + Hash + Ord + Display,
{
  fn cmp(&self, other: &Self) -> Ordering {
    if self.len() == other.len() {
      for idx in 0..self.len() {
        let r = self.get(idx).cmp(&other.get(idx));
        if r == Equal {
          continue;
        } else {
          return r;
        }
      }
      Equal
    } else {
      self.len().cmp(&other.len())
    }
  }
}

impl<T> PartialOrd for FingerList<T>
where
  T: Debug + Clone + Ord + Display + Hash,
{
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl<T> Eq for FingerList<T> where T: Debug + Clone + Ord + Display + Hash {}

impl<T> PartialEq for FingerList<T>
where
  T: Debug + Clone + Ord + Display + Hash,
{
  fn eq(&self, other: &Self) -> bool {
    if self.len() == other.len() {
      for idx in 0..self.len() {
        if self.get(idx) != other.get(idx) {
          return false;
        }
      }
      true
    } else {
      false
    }
  }
}

impl<T> Measured for FingerList<T>
where
  T: Debug + Clone,
{
  type Measure = Sum<usize>;

  fn measure(&self) -> Self::Measure {
    Sum(1)
  }
}

impl<'a, T> Index<usize> for FingerList<T>
where
  T: Clone + Eq + PartialEq + Debug + Ord + PartialOrd + Hash,
{
  type Output = T;

  fn index<'b>(&self, idx: usize) -> &Self::Output {
    match self.0.find(|m| **m > idx) {
      Some(value) => value,
      None => unreachable!("out of bound"),
    }
  }
}

impl<T> FingerList<T>
where
  T: Debug + Clone + Ord + Display + Hash,
{
  pub fn get(&self, idx: usize) -> Option<&T> {
    self.0.find(|m| **m > idx).map(|value| &**value)
  }

  pub fn len(&self) -> usize {
    match self.0.measure() {
      Sum(s) => s,
    }
  }

  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }

  pub fn push(&self, item: T) -> Self {
    let next = self.0.push_right(Size(item));
    Self(next)
  }
  pub fn rest(&self) -> Result<Self, String> {
    let (_, right) = self.0.split(|measure| *measure > Sum(1));
    Ok(Self(right))
  }

  pub fn butlast(&self) -> Result<Self, String> {
    let (_, left) = self.0.view_right().unwrap();
    Ok(Self(left))
  }
  pub fn unshift(&self, item: T) -> Self {
    let next = self.0.push_left(Size(item));
    Self(next)
  }
  pub fn slice(&self, from: usize, to: usize) -> Result<Self, String> {
    let (_, right) = self.0.split(|measure| *measure > Sum(from));
    let (next, _) = right.split(|measure| *measure > Sum(to - from));
    Ok(Self(next))
  }
  pub fn reverse(&self) -> Self {
    let mut xs: FingerTree<ArcRefs, Size<T>> = FingerTree::new();
    for y in (&self.0).into_iter() {
      xs = xs.push_left(y);
    }
    Self(xs)
  }

  pub fn skip(&self, from: usize) -> Result<Self, String> {
    self.slice(from, self.len())
  }

  pub fn assoc(&self, from: usize, item: T) -> Result<Self, String> {
    let (left, right) = self.0.split(|measure| *measure > Sum(from));
    let (_, r2) = right.split(|measure| *measure > Sum(1));
    // let (_, r2) = right.view_left().unwrap();
    let next = r2.push_left(Size(item));
    Ok(Self(left.concat(&next).to_owned()))
  }

  pub fn dissoc(&self, from: usize) -> Result<Self, String> {
    let (left, right) = self.0.split(|measure| *measure > Sum(from));
    let (_, next) = right.view_left().unwrap();
    Ok(Self(left.concat(&next).to_owned()))
  }

  pub fn assoc_before(&self, from: usize, item: T) -> Result<Self, String> {
    let (left, right) = self.0.split(|measure| *measure > Sum(from));
    let next = right.push_left(Size(item));
    Ok(Self(left.concat(&next).to_owned()))
  }

  pub fn assoc_after(&self, from: usize, item: T) -> Result<Self, String> {
    let (left, right) = self.0.split(|measure| *measure > Sum(from + 1));
    let next = right.push_left(Size(item));
    Ok(Self(left.concat(&next).to_owned()))
  }

  pub fn from(xs: &[T]) -> Self {
    let ret: FingerTree<ArcRefs, _> = xs.iter().map(|x| Size(x.to_owned())).collect();
    Self(ret)
  }

  pub fn new_empty() -> Self {
    Self(FingerTree::new())
  }

  pub fn iter(&self) -> FigerListRefIntoIterator<T> {
    FigerListRefIntoIterator { value: self, index: 0 }
  }

  pub fn index_of(&self, item: &T) -> Option<usize> {
    for (idx, y) in (&self.0).into_iter().enumerate() {
      if item == &*y {
        return Some(idx);
      }
    }
    None
  }
}

// experimental code to turn `&FingerList<_>` into iterator
impl<'a, T> IntoIterator for &'a FingerList<T>
where
  T: Clone + Display + Eq + PartialEq + Debug + Ord + PartialOrd + Hash,
{
  type Item = &'a T;
  type IntoIter = FigerListRefIntoIterator<'a, T>;

  fn into_iter(self) -> Self::IntoIter {
    FigerListRefIntoIterator { value: self, index: 0 }
  }
}

pub struct FigerListRefIntoIterator<'a, T>
where
  T: Clone + Display + Eq + PartialEq + Debug + Ord + PartialOrd + Hash,
{
  value: &'a FingerList<T>,
  index: usize,
}

impl<'a, T> Iterator for FigerListRefIntoIterator<'a, T>
where
  T: Clone + Display + Eq + PartialEq + Debug + Ord + PartialOrd + Hash,
{
  type Item = &'a T;
  fn next(&mut self) -> Option<Self::Item> {
    if self.index < self.value.len() {
      // println!("get: {} {}", self.value.format_inline(), self.index);
      let idx = self.index;
      self.index += 1;
      Some(self.value.get(idx).unwrap())
    } else {
      None
    }
  }
}

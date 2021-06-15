import { Hash, overwriteHashGenerator, valueHash, mergeValueHash } from "@calcit/ternary-tree";
import { overwriteComparator, initTernaryTreeMap } from "@calcit/ternary-tree";
import { overwriteDataComparator } from "./js-map";

import { CalcitRecord, fieldsEqual } from "./js-record";
import { CalcitMap } from "./js-map";

import { CalcitValue } from "./js-primes";
import { CalcitList } from "./js-list";
import { CalcitSet } from "./js-set";
import { CalcitTuple } from "./js-tuple";

export class CalcitKeyword {
  value: string;
  cachedHash: Hash;
  constructor(x: string) {
    this.value = x;
    this.cachedHash = null;
  }
  toString() {
    return `:${this.value}`;
  }
}

export class CalcitSymbol {
  value: string;
  cachedHash: Hash;
  constructor(x: string) {
    this.value = x;
    this.cachedHash = null;
  }
  toString() {
    return `'${this.value}`;
  }
}

export class CalcitRecur {
  args: CalcitValue[];
  cachedHash: Hash;
  constructor(xs: CalcitValue[]) {
    this.args = xs;
    this.cachedHash = null;
  }

  toString() {
    return `(&recur ...)`;
  }
}

export let isNestedCalcitData = (x: CalcitValue): boolean => {
  if (x instanceof CalcitList) {
    return x.len() > 0;
  }
  if (x instanceof CalcitMap) {
    return x.len() > 0;
  }
  if (x instanceof CalcitRecord) {
    return x.fields.length > 0;
  }
  if (x instanceof CalcitSet) {
    return false;
  }
  return false;
};

export let tipNestedCalcitData = (x: CalcitValue): string => {
  if (x instanceof CalcitList) {
    return "'[]...";
  }
  if (x instanceof CalcitMap) {
    return "'{}...";
  }
  if (x instanceof CalcitRecord) {
    return "'%{}...";
  }
  if (x instanceof CalcitSet) {
    return "'#{}...";
  }
  return x.toString();
};

export class CalcitRef {
  value: CalcitValue;
  path: string;
  listeners: Map<CalcitValue, CalcitFn>;
  cachedHash: Hash;
  constructor(x: CalcitValue, path: string) {
    this.value = x;
    this.path = path;
    this.listeners = new Map();
    this.cachedHash = null;
  }
  toString(): string {
    return `(&ref ${this.value.toString()})`;
  }
}

export type CalcitFn = (...xs: CalcitValue[]) => CalcitValue;

export let getStringName = (x: CalcitValue): string => {
  if (typeof x === "string") {
    return x;
  }
  if (x instanceof CalcitKeyword) {
    return x.value;
  }
  if (x instanceof CalcitSymbol) {
    return x.value;
  }
  throw new Error("Cannot get string as name");
};

/** returns -1 when not found */
export function findInFields(xs: Array<string>, y: string): number {
  let lower = 0;
  let upper = xs.length - 1;

  while (upper - lower > 1) {
    let pos = (lower + upper) >> 1;
    let v = xs[pos];
    if (y < v) {
      upper = pos - 1;
    } else if (y > v) {
      lower = pos + 1;
    } else {
      return pos;
    }
  }

  if (y == xs[lower]) return lower;
  if (y == xs[upper]) return upper;
  return -1;
}

var keywordRegistery: Record<string, CalcitKeyword> = {};

export let kwd = (content: string) => {
  let item = keywordRegistery[content];
  if (item != null) {
    return item;
  } else {
    let v = new CalcitKeyword(content);
    keywordRegistery[content] = v;
    return v;
  }
};

export var refsRegistry = new Map<string, CalcitRef>();

let defaultHash_nil = valueHash("nil:");
let defaultHash_number = valueHash("number:");
let defaultHash_string = valueHash("string:");
let defaultHash_keyword = valueHash("keyword:");
let defaultHash_true = valueHash("true:");
let defaultHash_false = valueHash("false:");
let defaultHash_symbol = valueHash("symbol:");
let defaultHash_fn = valueHash("fn:");
let defaultHash_ref = valueHash("ref:");
let defaultHash_tuple = valueHash("tuple:");
let defaultHash_set = valueHash("set:");
let defaultHash_list = valueHash("list:");
let defaultHash_map = valueHash("map:");

let fnHashCounter = 0;

let hashFunction = (x: CalcitValue): Hash => {
  if (x == null) {
    return defaultHash_nil;
  }
  if (typeof x === "number") {
    return mergeValueHash(defaultHash_number, x);
  }
  if (typeof x === "string") {
    return mergeValueHash(defaultHash_string, x);
  }
  // dirty solution of caching, trying to reduce cost
  if ((x as any).cachedHash != null) {
    return (x as any).cachedHash;
  }
  if (x instanceof CalcitKeyword) {
    let h = mergeValueHash(defaultHash_keyword, x.value);
    x.cachedHash = h;
    return h;
  }
  if (x === true) {
    return defaultHash_true;
  }
  if (x === false) {
    return defaultHash_false;
  }
  if (x instanceof CalcitSymbol) {
    let h = mergeValueHash(defaultHash_symbol, x.value);
    x.cachedHash = h;
    return h;
  }
  if (typeof x === "function") {
    fnHashCounter = fnHashCounter + 1;
    let h = mergeValueHash(defaultHash_fn, fnHashCounter);
    (x as any).cachedHash = h;
    return h;
  }
  if (x instanceof CalcitRef) {
    let h = mergeValueHash(defaultHash_ref, x.path);
    x.cachedHash = h;
    return h;
  }
  if (x instanceof CalcitTuple) {
    let base = defaultHash_tuple;
    base = mergeValueHash(base, hashFunction(x.fst));
    base = mergeValueHash(base, hashFunction(x.snd));
    x.cachedHash = base;
    return base;
  }
  if (x instanceof CalcitSet) {
    // TODO not using dirty solution for code
    let base = defaultHash_set;
    for (let item of x.value) {
      base = mergeValueHash(base, hashFunction(item));
    }
    return base;
  }
  if (x instanceof CalcitList) {
    let base = defaultHash_list;
    for (let item of x.items()) {
      base = mergeValueHash(base, hashFunction(item));
    }
    x.cachedHash = base;
    return base;
  }
  if (x instanceof CalcitMap) {
    let base = defaultHash_map;
    for (let [k, v] of x.pairs()) {
      base = mergeValueHash(base, hashFunction(k));
      base = mergeValueHash(base, hashFunction(v));
    }
    x.cachedHash = base;
    return base;
  }
  throw new Error("Unknown data for hashing");
};

// Dirty code to change ternary-tree behavior
overwriteHashGenerator(hashFunction);

export let toString = (x: CalcitValue, escaped: boolean): string => {
  if (x == null) {
    return "nil";
  }
  if (typeof x === "string") {
    if (escaped) {
      // turn to visual string representation
      if (/[\)\(\s\"]/.test(x)) {
        return JSON.stringify("|" + x);
      } else {
        return "|" + x;
      }
    } else {
      return x;
    }
  }
  if (typeof x === "number") {
    return x.toString();
  }
  if (typeof x === "boolean") {
    return x.toString();
  }
  if (typeof x === "function") {
    return `(&fn ...)`;
  }
  if (x instanceof CalcitSymbol) {
    return x.toString();
  }
  if (x instanceof CalcitKeyword) {
    return x.toString();
  }
  if (x instanceof CalcitList) {
    return x.toString();
  }
  if (x instanceof CalcitMap) {
    return x.toString();
  }
  if (x instanceof CalcitSet) {
    return x.toString();
  }
  if (x instanceof CalcitRecord) {
    return x.toString();
  }
  if (x instanceof CalcitRef) {
    return x.toString();
  }
  if (x instanceof CalcitTuple) {
    return x.toString();
  }

  console.warn("Unknown structure to string, better use `console.log`", x);
  return `${x}`;
};

export let to_js_data = (x: CalcitValue, addColon: boolean = false): any => {
  if (x == null) {
    return null;
  }
  if (x === true || x === false) {
    return x;
  }
  if (typeof x === "string") {
    return x;
  }
  if (typeof x === "number") {
    return x;
  }
  if (typeof x === "function") {
    return x;
  }
  if (x instanceof CalcitKeyword) {
    if (addColon) {
      return `:${x.value}`;
    }
    return x.value;
  }
  if (x instanceof CalcitSymbol) {
    if (addColon) {
      return `:${x.value}`;
    }
    return Symbol(x.value);
  }
  if (x instanceof CalcitList) {
    var result: any[] = [];
    for (let item of x.items()) {
      result.push(to_js_data(item, addColon));
    }
    return result;
  }
  if (x instanceof CalcitMap) {
    let result: Record<string, CalcitValue> = {};
    for (let [k, v] of x.pairs()) {
      var key = to_js_data(k, addColon);
      result[key] = to_js_data(v, addColon);
    }
    return result;
  }
  if (x instanceof CalcitSet) {
    let result = new Set();
    x.value.forEach((v) => {
      result.add(to_js_data(v, addColon));
    });
    return result;
  }
  if (x instanceof CalcitRecord) {
    let result: Record<string, CalcitValue> = {};
    for (let idx in x.fields) {
      result[x.fields[idx]] = to_js_data(x.values[idx]);
    }
    return result;
  }
  if (x instanceof CalcitRef) {
    throw new Error("Cannot convert ref to plain data");
  }
  if (x instanceof CalcitRecur) {
    throw new Error("Cannot convert recur to plain data");
  }

  return x;
};

export let _$n_map_$o_get = function (xs: CalcitValue, k: CalcitValue) {
  if (arguments.length !== 2) {
    throw new Error("map &get takes 2 arguments");
  }

  if (xs instanceof CalcitMap) return xs.get(k);

  throw new Error("Does not support `&get` on this type");
};

export let _$n__$e_ = (x: CalcitValue, y: CalcitValue): boolean => {
  if (x === y) {
    return true;
  }
  if (x == null) {
    if (y == null) {
      return true;
    }
    return false;
  }

  let tx = typeof x;
  let ty = typeof y;

  if (tx !== ty) {
    return false;
  }

  if (tx === "string") {
    return (x as string) === (y as string);
  }
  if (tx === "boolean") {
    return (x as boolean) === (y as boolean);
  }
  if (tx === "number") {
    return x === y;
  }
  if (tx === "function") {
    // comparing functions by reference
    return x === y;
  }
  if (x instanceof CalcitKeyword) {
    if (y instanceof CalcitKeyword) {
      return x === y;
    }
    return false;
  }
  if (x instanceof CalcitSymbol) {
    if (y instanceof CalcitSymbol) {
      return x.value === y.value;
    }
    return false;
  }
  if (x instanceof CalcitList) {
    if (y instanceof CalcitList) {
      if (x.len() !== y.len()) {
        return false;
      }
      let size = x.len();
      for (let idx = 0; idx < size; idx++) {
        let xItem = x.get(idx);
        let yItem = y.get(idx);
        if (!_$n__$e_(xItem, yItem)) {
          return false;
        }
      }
      return true;
    }
    return false;
  }
  if (x instanceof CalcitMap) {
    if (y instanceof CalcitMap) {
      if (x.len() !== y.len()) {
        return false;
      }
      for (let [k, v] of x.pairs()) {
        if (!y.contains(k)) {
          return false;
        }
        if (!_$n__$e_(v, _$n_map_$o_get(y, k))) {
          return false;
        }
      }
      return true;
    }
    return false;
  }
  if (x instanceof CalcitRef) {
    if (y instanceof CalcitRef) {
      return x.path === y.path;
    }
    return false;
  }
  if (x instanceof CalcitTuple) {
    if (y instanceof CalcitTuple) {
      return _$n__$e_(x.fst, y.fst) && _$n__$e_(x.snd, y.snd);
    }
    return false;
  }
  if (x instanceof CalcitSet) {
    if (y instanceof CalcitSet) {
      if (x.len() !== y.len()) {
        return false;
      }
      for (let v of x.value) {
        let found = false;
        // testing by doing iteration is O(n2), could be slow
        // but Set::contains does not satisfy here
        for (let yv of y.value) {
          if (_$n__$e_(v, yv)) {
            found = true;
            break;
          }
        }
        if (found) {
          continue;
        } else {
          return false;
        }
      }
      return true;
    }
    return false;
  }
  if (x instanceof CalcitRecur) {
    if (y instanceof CalcitRecur) {
      console.warn("Do not compare Recur");
      return false;
    }
    return false;
  }
  if (x instanceof CalcitRecord) {
    if (y instanceof CalcitRecord) {
      if (x.name !== y.name) {
        return false;
      }
      if (!fieldsEqual(x.fields, y.fields)) {
        return false;
      }
      if (x.values.length !== y.values.length) {
        return false;
      }
      for (let idx in x.fields) {
        if (!_$n__$e_(x.values[idx], y.values[idx])) {
          return false;
        }
      }
      return true;
    }
    return false;
  }
  throw new Error("Missing handler for this type");
};

// overwrite internary comparator of ternary-tree
overwriteComparator(_$n__$e_);
overwriteDataComparator(_$n__$e_);

use std::cell::RefCell;
use std::collections::HashMap;

struct Inner {
    strings: Vec<String>,
    symbols: HashMap<String, Symbol>,
}

pub(crate) struct StringInterner {
    inner: RefCell<Inner>,
    pub(crate) unit_symbol: Symbol,
    pub(crate) bottom_symbol: Symbol,
    pub(crate) bool_symbol: Symbol,
    pub(crate) u32_symbol: Symbol,
    pub(crate) u64_symbol: Symbol,
    pub(crate) i32_symbol: Symbol,
    pub(crate) i64_symbol: Symbol,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Symbol(pub(crate) u32);

impl StringInterner {
    pub(crate) fn new() -> Self {
        let mut si = Self {
            inner: RefCell::new(Inner {
                strings: Vec::new(),
                symbols: HashMap::new(),
            }),
            unit_symbol: Symbol(0),
            bottom_symbol: Symbol(0),
            bool_symbol: Symbol(0),
            u32_symbol: Symbol(0),
            u64_symbol: Symbol(0),
            i32_symbol: Symbol(0),
            i64_symbol: Symbol(0),
        };
        si.unit_symbol = si.intern("Unit");
        si.bottom_symbol = si.intern("Bottom");
        si.bool_symbol = si.intern("Bool");
        si.u32_symbol = si.intern("U32");
        si.u64_symbol = si.intern("U64");
        si.i32_symbol = si.intern("I32");
        si.i64_symbol = si.intern("I64");
        si
    }

    pub(crate) fn intern(&self, string: &str) -> Symbol {
        let mut inner = self.inner.borrow_mut();
        if let Some(&id) = inner.symbols.get(string) {
            return id;
        }
        let id = Symbol(inner.strings.len() as u32);
        inner.strings.push(string.to_owned());
        inner.symbols.insert(string.to_owned(), id);
        id
    }

    pub(crate) fn resolve(&self, id: Symbol) -> Option<String> {
        self.inner.borrow().strings.get(id.0 as usize).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // If we intern a string, then lookup the string by the resulting id, we must get the original string back.
    #[test]
    fn requirement_1() {
        let s = "foo";
        let interner = StringInterner::new();
        let symbol = interner.intern(s);
        assert_eq!(s, interner.resolve(symbol).unwrap());
    }

    // If two strings are equal, then they should have the same id.
    #[test]
    fn requirement_2() {
        let s1 = "foo";
        let s2 = "foo";
        let interner = StringInterner::new();
        assert_eq!(interner.intern(s1), interner.intern(s2));
    }
}

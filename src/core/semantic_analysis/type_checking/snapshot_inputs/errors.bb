const WRONG: Bool = 5;
const UNKNOWN: Foo = 5;

func mismatches(x: I32) -> Bool {
    let a: Bool = 1;
    let b = x + true;
    if x { 1 } else { false };
    x
}

func names() {
    let a = missing;
    let b: Baz = 1;
    missing_function(1);
}

func calls(x: I32) {
    x(1);
    calls(1, 2);
    calls(true);
    calls();
}

func operators(c: Bool) {
    let a = -c;
    let b = c + c;
    let d = not 1;
    let e = 1 and c;
}

func control_flow() -> I32 {
    break;
    continue;
    while true {
        break 1;
    }
    return true;
}

const RETURNS: I32 = return 1;

func assignments() {
    assignments = 1;
    1 = 2;
}

func no_tail() -> I32 {
    let x = 1;
}

func if_without_else(c: Bool) {
    if c { 1 }
}

func unknown_return() -> Missing {
    5
}

func poisoned() {
    let a = missing + 1;
    let b = missing();
    let c: I32 = missing;
}

func negations(u: U64) {
    let a = -u;
    let b: U32 = -5;
    let c = -1;
    let d: U64 = c;
    let e = -2;
}

func rustc_style(c: Bool) -> I32 {
    let a: I32 = if c { 1 };
    let b = 1;
    let d: Bool = b;
    return;
}

func wrong_tail() -> I32 {
    true
}

func wrong_return(c: Bool) -> I32 {
    if c {
        return false;
    }
    0
}

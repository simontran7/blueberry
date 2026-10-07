func f(c: Bool) -> U32 {
    let a = if c { 1 } else { 2 };
    let b: I64 = if c { 1 } else { 2 };
    if c {
        let unused = 0;
    }
    let chained = if c { 1 } else if not c { 2 } else { 3 };
    if c { a } else { 0 }
}

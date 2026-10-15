func pick(c: Bool, a: I32, b: I32) -> I32 {
    if c { a } else { b }
}

func without_else(c: Bool) -> I32 {
    let mut x = 1;
    if c {
        x = 2;
    }
    x
}

func early(c: Bool) -> I32 {
    if c {
        return 1;
    }
    2
}

func both_return(c: Bool) -> I32 {
    if c { return 1 } else { return 2 }
}

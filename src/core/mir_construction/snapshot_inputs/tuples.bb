func swap(pair: (I32, Bool)) -> (Bool, I32) {
    (pair.1, pair.0)
}

func nested() -> I64 {
    let mut point: (I64, (I64, I64)) = (1, (2, 3));
    point.1.0 = 4;
    point.0 + point.1.0
}

func with_units(mixed: ((), I32, ())) -> I32 {
    let units = ((), ());
    let same = mixed.0;
    mixed.1
}

func in_branches(c: Bool) -> (I32, Bool) {
    let mut pair = (0, false);
    if c {
        pair.0 = 1;
    } else {
        pair.1 = true;
    }
    pair
}

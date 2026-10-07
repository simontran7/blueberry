func early(c: Bool) -> I32 {
    if c {
        return 1;
    }
    2
}

func only_return() -> I32 {
    return 5;
}

func diverging_tail() -> Bool {
    let x = 1;
    return true
}

func unit_return() {
    return;
}

func diverging_branches(c: Bool) -> I32 {
    let never = if c { return 1 } else { return 2 };
    let later = if c { return 3 } else { return 4 };
    let used: U64 = later;
    0
}

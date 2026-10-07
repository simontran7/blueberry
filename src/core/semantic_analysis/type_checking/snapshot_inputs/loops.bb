func f() -> I32 {
    let x = loop {
        break 5;
    };
    let y: U64 = loop {
        if true {
            break 1;
        }
    };
    let mut i = 0;
    while i < 10 {
        i = i + 1;
        if i == 5 {
            continue;
        }
    }
    let never = loop {};
    x
}

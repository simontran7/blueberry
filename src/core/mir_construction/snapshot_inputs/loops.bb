func count(n: I32) -> I32 {
    let mut i = 0;
    let mut total = 0;
    while i < n {
        i = i + 1;
        if i == 3 {
            continue;
        }
        total = total + i;
    }
    total
}

func break_value() -> I32 {
    let mut i = 0;
    loop {
        i = i + 1;
        if i > 10 {
            break i;
        }
    }
}

func forever() -> I32 {
    loop {}
}

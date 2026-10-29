func swap(pair: (I32, Bool)) -> (Bool, I32) {
    (pair.1, pair.0)
}

func construction() {
    let empty: () = ();
    let one = (1,);
    let inferred = (1, true);
    let expected: (U64, (Bool, I64)) = (1, (true, 2));
    let grouped: (I32) = 1;
    let first = expected.1.1;
}

func assignment() {
    let mut point = (1, (2, 3));
    point.0 = 4;
    point.1.0 = 5;
    point.1 = (6, 7);
}

func diverging() -> (I32, I32) {
    (return (1, 2), 3)
}

func errors(pair: (I32, Bool), n: I32) {
    let a: (I32, Bool) = (true, 1);
    let b: (I32, I32) = (1, 2, 3);
    let c = pair.2;
    let d = pair.01;
    let e = pair.name;
    let f = n.0;
    let g = 1.0;
    let h = swap.0;
    let i: (I32, Foo) = (1, 2);
    let j: (shapes::Point, I32) = (1, 2);
    swap(pair).0 = 1;
}

func wrong_lengths() {
    let a: (U8, Bool) = (true, 1, false);
    let b: (I32,) = (1, 2);
}

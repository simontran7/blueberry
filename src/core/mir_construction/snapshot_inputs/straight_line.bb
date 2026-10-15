func add(a: I64, b: I64) -> I64 {
    let sum = a + b;
    -sum * 2
}

func nothing() {}

func unit_parameter(u: Unit, x: I32) -> I32 {
    let same = u == ();
    x
}

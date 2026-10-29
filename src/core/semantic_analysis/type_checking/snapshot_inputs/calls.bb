func add(a: I64, b: I64) -> I64 {
    a + b
}

func nothing() {}

func main() {
    let sum = add(1, 2);
    let unit = nothing();
    let function = add;
    let again = function(sum, 3);
}

func diverging_callee() -> I32 {
    (return 1)(2)
}

func unknown_callee() {
    let f = loop {};
    f(1);
}

func not_a_function() {
    let x = 5;
    x(true);
}

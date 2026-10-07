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

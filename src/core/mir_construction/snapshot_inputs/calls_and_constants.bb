const LIMIT: I32 = 10;

const DOUBLED: I32 = LIMIT * 2;

func add(a: I32, b: I32) -> I32 {
    a + b
}

func nothing() {}

func main() -> I32 {
    nothing();
    let function = add;
    let direct = add(1, LIMIT);
    function(direct, DOUBLED)
}

func nested() -> I32 {
    const LOCAL: I32 = 5;

    func helper(x: I32) -> I32 {
        x + LOCAL
    }

    helper(1)
}

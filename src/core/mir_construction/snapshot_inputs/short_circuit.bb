func both(a: Bool, b: Bool) -> Bool {
    a and b
}

func either(a: Bool, b: Bool) -> Bool {
    a or b
}

func diverging_rhs(a: Bool) -> Bool {
    a and return false
}

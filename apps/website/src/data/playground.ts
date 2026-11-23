export interface PlaygroundExample {
  id: string
  name: string
  description: string
  source: string
}

export const playgroundExamples: PlaygroundExample[] = [
  {
    id: 'hello',
    name: 'Hello, Nova',
    description: 'Structs, loops and string interpolation.',
    source: `import std.io { println }

struct User {
    name: String
    age: Int
}

fn describe(user: User) -> String {
    return "\${user.name} is \${user.age}"
}

fn main() {
    let user = User { name: "Ada", age: 36 }
    var total = 0
    for i in 0..4 {
        total += i
    }
    println("Hello, Nova!")
    println(describe(user))
    println(total)
}
`
  },
  {
    id: 'option',
    name: 'Option',
    description: 'A checked lookup and exhaustive match.',
    source: `import std.io { println }

fn find(items: List<Int>, target: Int) -> Option<Int> {
    var index = 0
    for item in items {
        if item == target {
            return Some(index)
        }
        index += 1
    }
    return None
}

fn main() {
    let found = find([3, 5, 8], 5)
    match found {
        Some(index) => println("found at \${index}"),
        None => println("not found")
    }
}
`
  },
  {
    id: 'result',
    name: 'Result',
    description: 'Error propagation without exceptions.',
    source: `import std.io { println }

fn divide(left: Int, right: Int) -> Result<Int, String> {
    if right == 0 {
        return Err("division by zero")
    }
    return Ok(left / right)
}

fn main() {
    match divide(12, 3) {
        Ok(value) => println("result: \${value}"),
        Err(message) => println(message)
    }
}
`
  },
  {
    id: 'loops',
    name: 'Loops',
    description: 'Exclusive ranges, inclusive ranges and while loops.',
    source: `import std.io { println }

fn main() {
    var total = 0
    for i in 0..=4 {
        total += i
    }

    var count = 0
    while count < 3 {
        count += 1
    }

    println("total=\${total} count=\${count}")
}
`
  },
  {
    id: 'diagnostic',
    name: 'Find a diagnostic',
    description: 'A deliberate type error to explore the checker.',
    source: `import std.io { println }

fn main() {
    let count: Int = "not a number"
    println(count)
}
`
  }
]

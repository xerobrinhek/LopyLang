# LopyLang Compiler

**LopyLang** — статически типизированный компилируемый язык программирования.

- Синтаксис: **Java-стиль для классов**, **Rust-стиль для функций**
- Компиляция в **нативный бинарник** через LLVM
- **ООП**: классы, поля, методы, конструкторы, инкапсуляция (`public`/`private`)
- **Импорты** из папок: `import mylib::*;`
- **Управляющие конструкции**: `if`, `while`, `for`
- **Ввод/вывод**: `println(...)`, `input(int|string|bool)`
- Конкатенация строк и строк с числами
- Сравнение строк

---

## Пример

```rust
import mylib::*;

fn main() {
    let calc = new Calculator();
    let sum = calc.add(5, 3);
    println("5 + 3 =", sum);
    
    let greeter = new Greeter("Hello");
    let message = greeter.greet("Artem");
    println(message);
}
```

## Сборка

```bash
cargo build --release
```

## Запуск

```bash
./target/release/lopy main.lp --run
```

## Лицензия

GNU General Public License v3.0

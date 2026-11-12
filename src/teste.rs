struct Pessoa {
    nome: String,
    idade: i16,
}

fn main() {
    let pessoa = Pessoa {
        nome: String::from("Pedro"),
        idade: 45,
    };

    println!("Nome: {}", pessoa.nome);
    println!("Idade: {}", pessoa.idade);
}

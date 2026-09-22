use rand::seq::SliceRandom;
use rodio::{Decoder, MixerDeviceSink, Player};
use std::{
    env,
    fs::{self, File},
    io::{self, BufReader, Write},
    ops::ControlFlow::Break,
    path::{Path, PathBuf},
    process::Command,
    sync::{mpsc, Arc, Mutex},
    thread::{self},
    time::Duration,
};
use trash;

fn main() -> io::Result<()> {
    let path: String;
    println!("Aperte [Enter] para continuar OU digite qualquer tecla para selecionar o diretorio:");
    let user_input = get_user_input();
    if user_input.is_empty() {
        path = fs::read_to_string("arquivo.txt").unwrap();
    } else {
        println!("digite o diretorio");

        let input = get_user_input();

        match fs::write("arquivo.txt", &input) {
            Ok(_) => println!("Arquivo criado com sucesso!"),
            Err(e) => println!("Erro ao criar o arquivo: {}", e),
        }
        path = input;
    }

    let mut files = fs::read_dir(path)?
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, io::Error>>()?;

    files = shuffle_files(files);

    let sink_handle =
        rodio::DeviceSinkBuilder::open_default_sink().expect("open default audio stream");

    let (tx, rx) = mpsc::channel::<String>();

    let playback_thread = thread::spawn(move || {
        playback_loop(rx, files, sink_handle);
    });

    input_lopp(tx);

    playback_thread.join().unwrap();
    Ok(())
}

fn playback_loop(
    rx: mpsc::Receiver<String>,
    mut files: Vec<PathBuf>,
    sink_handle: MixerDeviceSink,
) {
    let mut idx = 0;

    loop {
        if idx >= files.len() {
            println!("Não há mais músicas na lista.");
            break;
        }

        let (player, _) = play_music(&files[idx], &sink_handle);

        loop {
            if player.empty() {
                idx += 1;
                break;
            }

            if let Ok(command) = rx.try_recv() {
                match command.as_str() {
                    "next" => {
                        if idx + 1 < files.len() {
                            idx += 1;
                        } else {
                            println!("Já estamos na última música.");
                        }
                        break;
                    }
                    "prev" => {
                        if idx > 0 {
                            idx -= 1;
                        } else {
                            println!("Já estamos na primeira música.");
                        }
                        break;
                    }
                    "play" => {
                        player.play();
                        println!("musica tocando");
                    }
                    "pause" => {
                        player.pause();
                        println!("musica pausada")
                    }
                    "delete" => {
                        let file_to_delete = &files[idx];
                        let _ = trash::delete(file_to_delete);
                        println!("Enviado para a lixeira: {:?}", file_to_delete);

                        files.remove(idx);
                        if idx >= files.len() && idx > 0 {
                            idx -= 1;
                        }
                        break;
                    }
                    "quit" => {
                        println!("Saindo do programa.");
                        return;
                    }
                    _ => println!("Comando inválido."),
                }
            }
            thread::sleep(Duration::from_millis(500));
        }
    }
}

fn input_lopp(tx: mpsc::Sender<String>) {
    loop {
        let user_input = get_user_input();
        let command = match user_input.as_str() {
            "d" => "next",
            "a" => "prev",
            "w" => "play",
            "s" => "pause",
            "D" => "delete",
            "q" => "quit",
            _ => continue,
        };
        tx.send(command.to_string()).unwrap();
        if command == "quit" {
            break;
        }
    }
}

fn play_music(file_path: &PathBuf, sink_handle: &MixerDeviceSink) -> (Player, io::Result<()>) {
    println!("Digite 'd' próximo, 'a' anterior, 'w' tocar, 's' pausar, 'D' deletar ou 'q' sair: ");
    let file = match File::open(file_path) {
        Ok(f) => BufReader::new(f),
        Err(e) => {
            eprintln!("Erro ao abrir a música: {}", e);
            let empty_player = Player::connect_new(sink_handle.mixer());
            return (empty_player, Err(e));
        }
    };
    let player = rodio::play(&sink_handle.mixer(), file).unwrap();
    println!("Tocando arquivo: {:?}", file_path.file_name());
    (player, Ok(()))
}

fn get_user_input() -> String {
    io::stdout().flush().expect("Falha ao limpar o buffer");
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Falha ao ler a linha");
    input.trim().to_string()
}

fn shuffle_files(mut files: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut rng = rand::thread_rng();
    files.shuffle(&mut rng);
    files
}

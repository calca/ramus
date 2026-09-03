# Rimuovere l'ultimo `.expect()` in codice di produzione

Stato: proposta, da implementare. La più piccola delle spec di
"production readiness" — un solo punto, nessuna domanda aperta.

## Motivazione

CLAUDE.md, stile di codice: "mai `unwrap()` o `expect()` nei command e
nel codice di produzione del core. Nei test è ammesso." L'unico punto
rimasto fuori dai test in tutto il workspace (verificato con `grep`
su `crates/`/`src-tauri/src`, escludendo `#[cfg(test)]`):
`crates/ramus-mcp/src/main.rs:53`, in `print_config()`:

```rust
serde_json::to_string_pretty(&snippet).expect("serializzazione JSON statica")
```

`snippet` è un `serde_json::Value` costruito da una macro `json!()`
con solo stringhe (incluso `exe.to_string_lossy()`, sempre valida
UTF-8-rappresentabile) — questo `expect()` non può realisticamente
mai panicare, ma la regola di CLAUDE.md non fa eccezioni per "casi
che non possono succedere davvero": lo stile del progetto è zero
`unwrap`/`expect` fuori dai test, punto.

(Il secondo caso trovato, `src-tauri/src/lib.rs:144`
`.expect("error while running tauri application")`, è il boilerplate
standard generato da Tauri stesso per l'avvio dell'app — un fallimento
lì è un errore di bootstrap del framework senza percorso di recupero
sensato, pattern accettato anche in produzione da Tauri stesso. Non
in scope per questa spec.)

## Modifica

`print_config()` cambia firma da `fn print_config()` a `fn
print_config() -> Result<(), serde_json::Error>`, propaga l'errore con
`?` invece di `.expect(...)`. Il chiamante (`main()`, dove
`--print-config` viene gestito) stampa l'errore su stderr ed esce con
`std::process::exit(1)` in caso di fallimento — stesso pattern già in
uso in `main()` per `mcp_disabled_message` (M5).

## Fuori scope

Riscrivere `print_config()` per qualunque altro motivo: solo la
gestione dell'errore cambia, nessun altro comportamento.

## Verifica

`cargo test -p ramus-mcp`, `cargo clippy --all-targets -- -D
warnings`, `cargo fmt --check` — tutti puliti. `grep` di conferma:
zero `.unwrap()`/`.expect()` fuori da `#[cfg(test)]` in tutto il
workspace dopo la modifica.

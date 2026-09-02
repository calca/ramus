# specs/

Spec di lavoro, una per feature/task, create man mano che si procede.
`SPEC.md` alla radice resta la specifica di progetto complessiva e le
milestone; qui vanno gli approfondimenti puntuali prima di essere
implementati.

Organizzate in una sottocartella per milestone (`M1/`, `M2/`, `M3/`,
...), la stessa numerazione di `SPEC.md`. Una spec che tocca più
milestone va in quella a cui appartiene il suo pezzo principale.

Nome file: `YYYY-MM-DD-nome-spec.STATO.md`, con la data di creazione
della spec (non di modifica: se una spec viene rivista più avanti, il
nome non cambia) e `STATO` uno tra:

- `TODO` — proposta, non ancora implementata (o implementata solo in
  parte).
- `DONE` — implementata.

Il file si rinomina quando lo stato cambia (`TODO` → `DONE`); ogni
riferimento altrove nel repo (commenti nel codice, `SPEC.md`, altre
spec) va aggiornato di conseguenza.

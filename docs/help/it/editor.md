# L'editor (F4)

Righe, un blocco, annullamento su tutta la sessione, e un salvataggio che scrive un file nuovo
accanto a quello vecchio e lo rinomina sopra — così un salvataggio interrotto non lascia mai
mezzo file.

`F2` salva, `Shift+F2` salva con un altro nome, `F7` apre lo stesso dialogo di ricerca che ha
il visualizzatore — testo, esadecimale o espressione regolare, maiuscole, parole intere,
avanti o indietro — e `Ctrl+F7` sostituisce.
`Ctrl+Z` e `Ctrl+Y` annullano e rifanno. `F6` passa il file al visualizzatore alla stessa riga.

Un blocco a colonne è un rettangolo invece di una sequenza di righe: è così che si sposta una
colonna di numeri senza toccare quello che le sta accanto.

# I colori

Un file il cui nome dice cos'è — Rust, C, C++, Java, C#, JavaScript, TypeScript, PHP, Python,
Ruby, Go, shell, SQL, HTML, XML, CSS, Markdown, JSON, TOML, YAML, INI, Make, Dockerfile, Lua,
Perl — viene colorato mentre lo leggi, e la riga di stato nomina il linguaggio, così vedi
subito quando un file viene letto come qualcosa che non intendevi. I colori vengono dal tema,
quindi lo seguono. `ui.syntax = false` in `fcmd.toml` li spegne.

# Byte invece di righe

`F4` su un binario apre lo stesso editor sui suoi byte, e `Alt+H` passa da una parte
all'altra. Due riquadri, l'esadecimale e il testo; `Tab` si sposta fra i due e in entrambi si
scrive. `Insert` sceglie fra sovrascrivere e inserire — in un binario un byte spostato di solito
rompe il file, quindi parte in sovrascrittura.

I byte cambiati sono disegnati come cambiati fino a che non vengono scritti, con accanto
l'elenco di cosa è cambiato: posizione, byte vecchio, byte nuovo. `Ctrl+Z` ne riporta indietro
uno. `F2` scrive, in modo atomico.

`F3` marca un blocco e `F12` dice cosa farne: scriverlo in un file, leggere un file dentro,
riempirlo con un byte, farne lo XOR con una chiave, copiare o spostare byte da un'altra parte
dello stesso file, inserire zeri, cancellare. Ognuna dice quanto diventerà lungo il file prima
di farlo.

`F9` assembla un'istruzione dove sta il cursore — `ret`, `xor eax, eax`, `mov rax, 1`,
`jmp 0x401010`, `call rax`. Un salto si scrive come l'indirizzo a cui vuoi arrivare, non come
uno scostamento. Qualunque cosa venga codificata viene decodificata di nuovo prima di essere
mostrata, così quello che vedi è l'istruzione che vedrà il processore.

Gli indirizzi si scrivono come li stampa un disassemblato: `mov eax, [rbx]`, `[rbx+8]`,
`[rbx+rcx*4+8]`, `[0x401000]`, e `[rip+0x2e35]` con lo scostamento contato dalla fine
dell'istruzione, così una riga letta sullo schermo si può riscrivere tale e quale. Quando
nessun registro dice quanto è larga la cosa all'indirizzo, lo dice una parola di dimensione:
`mov dword [rbx], 1`, oppure `byte ptr` per chi scrive in masm. Senza, chiede invece di
indovinare, perché un byte e otto non sono la stessa patch. Quello che non fa — i prefissi di
segmento, i puntatori far, i registri in virgola mobile e vettoriali — lo rifiuta per nome.

`Alt+=` è la calcolatrice: una riga di aritmetica con la risposta in decimale, esadecimale,
ottale, binario e come caratteri tutto insieme. Legge i letterali che stampano gli strumenti
per binari — `0x1f`, `1Fh`, `0b1010`, `'PE'` — e lo stesso lettore prende i numeri scritti
nell'assemblatore e nelle operazioni sui blocchi.

# I dispositivi

Su Linux e macOS un disco è un file, quindi l'editor esadecimale apre `/dev/sdb` in sola
lettura e la riga di stato dice cos'è e quanto ha misurato. I byte si scrivono *fuori*, in un
file, mai di nuovo sopra il disco.

# Tabulazioni, spazi e i tasti meno ovvi

`F9 → Opzioni → Impostazioni dell'editor` ha le cinque che contano. **Tabulazioni** è una scelta
fra tre e non una casella: lasciarle come sono, trasformare in spazi una tabulazione appena
scritta, oppure espandere ogni tabulazione del file quando lo si apre — l'ultima cambia il
file, quindi dice quante righe ha toccato e il salvataggio successivo scrive quello. **Mostra
spazi e tabulazioni** disegna uno spazio come `·` e una tabulazione come `→` con il suo
riempimento, che è come un'indentazione mista diventa visibile. **Il cursore si ferma dopo il
testo trovato** serve per scrivere sopra quello che è stato trovato invece di leggerlo.

`Ctrl+Q` inserisce il tasto successivo per quello che è: un carattere di controllo, o una
tabulazione vera dove le tabulazioni vengono espanse. Non c'è altro modo di mettere nel file né
l'uno né l'altra.

`Alt+I` e `Alt+U` spostano il blocco — o, se non c'è niente di selezionato, la riga su cui sta
il cursore — di un'indentazione a destra e a sinistra. `Tab` continua a scrivere una
tabulazione, perché è il tasto che si preme mentre si scrive.

`Ctrl+Shift+Ins` aggiunge il blocco a quello che è già negli appunti invece di sostituirlo,
così si possono raccogliere pezzi da qua e da là e incollarli in una volta. `Ctrl+N` e `Ctrl+E`
mettono il cursore sulla prima e sull'ultima riga *dello schermo*, lasciando lo schermo dov'è.

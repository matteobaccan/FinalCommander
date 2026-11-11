# L'editor (F4)

Righe, un blocco, annullamento su tutta la sessione, e un salvataggio che scrive un file nuovo
accanto a quello vecchio e lo rinomina sopra — così un salvataggio interrotto non lascia mai
mezzo file.

`F2` salva, `Shift+F2` salva con un altro nome. `F6` passa il file al visualizzatore alla
stessa riga.

`F7` apre lo stesso dialogo di ricerca che ha il visualizzatore — testo, esadecimale o
espressione regolare, maiuscole, parole intere, avanti o indietro — con un pulsante in più.
**Tutte le occorrenze** elenca ogni riga che ne contiene una, con il suo numero: scrivendo si
restringe l'elenco, `Enter` va alla riga, e `Shift+F7` prosegue da lì come dopo qualsiasi
altra ricerca. *Solo parole intere* vuol dire che ciò che si trova non è un pezzo di una
parola più lunga: subito prima e subito dopo non c'è una lettera, una cifra o un trattino
basso. Vale per `set` in `set = 1` e non in `offset`, e funziona anche per qualcosa che una
parola non è, come `++`.

`Ctrl+F7` sostituisce, e chiede tutto in un solo dialogo: che cosa cercare, che cosa mettere
al suo posto, testo o espressione regolare, maiuscole, parole intere e *Mantieni lo stile*.

- In un'**espressione regolare** la sostituzione può usare ciò che i gruppi hanno catturato:
  `$1` è il primo gruppo, `${name}` uno con un nome, `$$` un dollaro. Si scrive `${1}x` quando
  al gruppo segue una lettera, perché `$1x` verrebbe letto come un gruppo di nome `1x`. In una
  ricerca di testo un dollaro è solo un dollaro.
- ***Mantieni lo stile*** dà a una sostituzione di testo le maiuscole di ciò che sostituisce:
  sostituendo `foo` con `bar` anche `Foo` diventa `Bar` e `FOO` diventa `BAR`. Le forme
  riconosciute sono tre — tutto minuscolo, tutto maiuscolo, iniziale maiuscola — e una parola
  scritta in un altro modo (`fooBar`) riceve la sostituzione esattamente come l'hai scritta.
  Conta solo quando la ricerca non distingue le maiuscole.
- **Sostituisci tutto** lavora su tutto il file, dice quante ne ha cambiate, ed è un solo
  `Ctrl+Z`. **Sostituisci il prossimo** cambia l'occorrenza su cui sta il cursore, o la
  successiva, e seleziona quella dopo: per scorrere un file un'occorrenza alla volta.

Il dialogo si apre su ciò che hai cercato l'ultima volta, e come. Ogni sostituzione eseguita
viene ricordata, e `Alt+Shift+F7` le elenca — prima quelle tenute con un nome, poi le recenti
— perché `Enter` le esegua di nuovo senza riscrivere un'espressione regolare; `Ins` tiene una
recente con un nome (finisce in `fcmd.toml` come blocco `[[editor.replace]]`), `Del` dimentica
una tenuta. Una di queste viene eseguita come il dialogo era impostato l'ultima volta: come
testo o come espressione, con o senza parole intere.

`Ctrl+Z` annulla e `Ctrl+Shift+Z` rifà. Una serie di tasti battuti è un passo solo, come in
FAR: la parola appena scritta se ne va con un `Ctrl+Z`, non lettera per lettera. Lo stesso
vale per una serie di `Backspace` e per una serie di `Del`. Un passo finisce quando fai
qualcos'altro — sposti il cursore, premi `Enter`, dai un comando, o passi dallo scrivere al
cancellare. L'editor tiene gli ultimi 1000 passi di ogni file; `editor.undo_depth` nelle
impostazioni (*Opzioni → Tutti i parametri*) cambia il numero per i file aperti da quel
momento.

Un blocco a colonne è un rettangolo invece di una sequenza di righe: è così che si sposta una
colonna di numeri senza toccare quello che le sta accanto.

# Fine riga, byte order mark e tabella codici

Un salvataggio scrive il file com'è stato letto: gli stessi fine riga, la stessa tabella
codici, il byte order mark se c'era, e nessun a capo dopo l'ultima riga se non c'era. La riga
di stato dice quali — `LF`, `CRLF` o `CR`, poi la tabella codici, poi `BOM` quando il file
comincia con quello.

`Shift+F8` è dove si cambia. L'elenco ha tre gruppi, e in ognuno la riga vera in questo
momento è segnata: il fine riga (LF, CRLF, CR), il byte order mark (scritto o no), e tutte le
tabelle codici, nell'ordine dell'elenco del visualizzatore. `Enter` su una riga converte;
scrivi per restringere l'elenco, che è il modo svelto di arrivare a una tabella fra quaranta.

Una conversione da sola non cambia nulla sul disco. Il file risulta modificato, la riga di
stato mostra la nuova risposta, e `F2` la scrive; `Ctrl+Z` la annulla, una conversione alla
volta, come ogni altra modifica.

- Un file con **fine riga misti** non ha nessuna delle tre righe segnata. Sceglierne una dà a
  tutte le righe lo stesso fine riga.
- Solo UTF-8 e UTF-16 hanno un **byte order mark**. Convertire in UTF-16 lo aggiunge, perché
  senza non c'è che un'ipotesi a distinguere i due ordini dei byte; convertire in una tabella
  che non ne ha lo toglie.
- Una **tabella codici** che non ha un byte per uno dei caratteri del file viene rifiutata, e
  non cambia nulla: `中` non sta in Windows-1252, e un salvataggio che lo trasformasse in `?`
  sarebbe una parola persa in silenzio.

Nel visualizzatore lo stesso tasto legge i byte con un'altra tabella. Qui il testo è già
letto, quindi la tabella che scegli è quella in cui verrà scritto.

# I colori

Un file il cui nome dice cos'è — Rust, C, C++, Java, C#, JavaScript, TypeScript, PHP, Python,
Ruby, Go, shell, SQL, HTML, XML, CSS, Markdown, JSON, TOML, YAML, INI, Make, Dockerfile, Lua,
Perl, Pascal e Delphi — viene colorato mentre lo leggi, e la riga di stato nomina il linguaggio, così vedi
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

Su Windows funziona lo stesso con i nomi che usa Windows: `\\.\PhysicalDrive0` per un disco e
`\\.\C:` per un volume, scritti a mano e non trovati in un pannello — il filesystem non li
elenca. La lunghezza la dice il dispositivo, perché non c'è niente da misurare con un seek, e
la lettura è fatta a settori interi perché è tutto quello che un handle così accetta. Servono
i diritti di amministratore, e se non ci sono lo dice.

# Parentesi, righe unite, righe ordinate

`Alt+B` salta alla parentesi che corrisponde a quella sotto il cursore, o a quella appena prima —
`()`, `[]`, `{}` e `<>`, anche su più righe — e una seconda volta torna indietro; è il tasto di
mcedit. Le parentesi dentro stringhe e commenti contano come le altre: questo è l'editor di un
file manager, non di un linguaggio. `Alt+J` unisce la riga successiva a questa, con la sua
indentazione ridotta a uno spazio. `Alt+Shift+S` ordina le righe del blocco, e niente fuori da
lì; `Alt+Shift+U` toglie ogni riga che ne ripete una sopra — la prima di ciascuna resta dov'era,
quindi il blocco non va ordinato prima — e dice quante ne ha tolte; `Alt+Shift+R` capovolge le
righe del blocco; `Alt+Shift+N` mette un numero davanti a ciascuna, da 1, allineato sul più
largo. `Alt+Shift+T` toglie spazi e tabulazioni alla fine di ogni riga. Ognuna si annulla con
un solo `Ctrl+Z`.

**Togli gli spazi a fine riga al salvataggio**, nelle impostazioni dell'editor, fa l'ultima a
ogni `F2`. È spenta per default, perché un salvataggio che cambia righe che nessuno ha toccato è
una differenza che nessuno ha chiesto.

# Tabulazioni, spazi e i tasti meno ovvi

`F9 → Opzioni → Impostazioni dell'editor` ha le sei che contano. **Tabulazioni** è una scelta
fra tre e non una casella: lasciarle come sono, trasformare in spazi una tabulazione appena
scritta, oppure espandere ogni tabulazione del file quando lo si apre — l'ultima cambia il
file, quindi dice quante righe ha toccato e il salvataggio successivo scrive quello. **Mostra
spazi e tabulazioni** disegna uno spazio come `·` e una tabulazione come `→` con il suo
riempimento, che è come un'indentazione mista diventa visibile. **Il cursore si ferma dopo il
testo trovato** serve per scrivere sopra quello che è stato trovato invece di leggerlo.

**Il cursore può andare oltre la fine della riga** è spenta per default. Accesa, le frecce
entrano nelle colonne a destra del testo invece di saltare alla riga successiva: è quello che
serve per allineare una colonna di numeri, o per marcare le stesse colonne su righe di
lunghezza diversa. Muoversi là non scrive niente: gli spazi compaiono al tasto che mette un
carattere in una di quelle colonne, dentro la stessa modifica, così un solo `Ctrl+Z` porta via
il carattere e il suo riempimento insieme. `Backspace` e `Del` là fuori spostano il cursore e
lasciano il file come sta.

`Ctrl+Q` inserisce il tasto successivo per quello che è: un carattere di controllo, o una
tabulazione vera dove le tabulazioni vengono espanse. Non c'è altro modo di mettere nel file né
l'uno né l'altra.

`Alt+I` e `Alt+U` spostano il blocco — o, se non c'è niente di selezionato, la riga su cui sta
il cursore — di un'indentazione a destra e a sinistra. `Tab` continua a scrivere una
tabulazione, perché è il tasto che si preme mentre si scrive.

`Ctrl+Shift+Ins` aggiunge il blocco a quello che è già negli appunti invece di sostituirlo,
così si possono raccogliere pezzi da qua e da là e incollarli in una volta. `Ctrl+N` e `Ctrl+E`
mettono il cursore sulla prima e sull'ultima riga *dello schermo*, lasciando lo schermo dov'è.

# Il `.editorconfig` di un progetto

Quando un file che apri ha un `.editorconfig` sopra di sé, quello che dice su quel file vince
sulle impostazioni: spazi o tabulazioni e quanto larghi (`indent_style`, `indent_size`,
`tab_width`), e se togliere gli spazi in coda al salvataggio (`trim_trailing_whitespace`). Per
un file nuovo anche il fine riga e l'a-capo finale (`end_of_line`, `insert_final_newline`); un
file che esiste già tiene quelli che ha.

# Finire una parola, e i caratteri che non si possono digitare

`Alt+/` finisce la parola prima del cursore con un'altra parola del file che comincia allo
stesso modo, prima la più vicina; ripremilo per la successiva, e dopo l'ultima torni a quello
che avevi scritto. `Alt+Shift+C` elenca i caratteri che la tastiera non ha — accenti, trattini e
virgolette, valute, frecce, matematica, le linee delle cornici di FAR, il greco — ognuno col suo
codice: scrivi `U+20AC` o una parte per restringere la lista, `Invio` per inserire, `Esc` per
tornare indietro.

# Togliere le righe che corrispondono

`Alt+Shift+D` chiede un'espressione regolare e toglie ogni riga che le corrisponde — nel blocco
se c'è, altrimenti in tutto il file. Comincia il pattern con `!` per tenere solo le righe che
corrispondono e togliere le altre: `!ERROR` lascia gli errori di un log. Un solo `Ctrl+Z` le
rimette tutte.

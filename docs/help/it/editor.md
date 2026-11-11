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

# Cercare mentre si scrive, e la parola sotto il cursore

`Ctrl+S` cerca mentre scrivi. Si apre una riga ai piedi del testo, e a ogni tasto il cursore
va al primo punto, a partire da dov'eri, che contiene ciò che hai scritto finora, e lo marca.
Aggiungi una lettera e la ricerca si restringe; cancellane una con `Backspace` e torna a ciò
che aveva trovato il testo più corto: cerca sempre da dove sei partito, quindi i tasti non ti
portano giù per il file.

| Tasto | Che cosa fa |
| --- | --- |
| `↓`, o di nuovo `Ctrl+S` | il punto successivo |
| `↑` | quello precedente |
| `Invio` | resta lì; `Shift+F7` e `Alt+F7` proseguono da quel punto |
| `Esc` | torna dov'eri, con il blocco che avevi marcato |

Quando nulla corrisponde, la riga di stato lo dice e il cursore torna dov'era all'inizio,
finché non cancelli una lettera. Cerca nel modo in cui è stata lasciata la finestra di `F7` —
come testo o come espressione regolare, con o senza maiuscole e parole intere — quindi è lì
che si cambia il modo; un'espressione non ancora finita semplicemente non è ancora trovata.

`Ctrl+F3` va al punto successivo in cui è scritta la **parola sotto il cursore**, e
`Ctrl+Shift+F3` a quello precedente, facendo il giro alla fine del file. È la parola come
parola intera e con le stesse maiuscole: da `count` non si ferma su `recount` né su `Count`.
Non viene chiesto nulla e la ricerca di `F7` resta com'era. Una parola che non è scritta
altrove, o un cursore che non è su una parola, viene detto nella riga di stato.

# Quando il file è cambiato sotto

Se qualcos'altro ha scritto il file mentre lo modificavi — una compilazione, un `git pull`, un
altro editor — `F2` non ci salva sopra. Chiede, e la domanda ha quattro risposte:

- **Sovrascrivi** scrive il tuo testo sopra quello che c'è adesso.
- **Ricarica** rilegge il file e butta le tue modifiche, che è anche quello che fa `Ctrl+R` in
  qualunque momento.
- **Cosa è cambiato** mette le due versioni fianco a fianco: a sinistra il file com'è sul
  disco, a destra il tuo testo con le modifiche non salvate, sulla stessa schermata con cui
  `Ctrl+Shift+D` confronta due file — `Alt+↓` e `Alt+↑` vanno da una differenza alla
  successiva, `W`, `C` ed `E` ignorano spazi, maiuscole e fine riga. Mostra soltanto: da lì
  non si unisce e non si scrive nulla. `Esc` la chiude e la domanda viene fatta di nuovo, ora
  che sai cosa farebbe perdere ogni risposta.
- **Annulla** torna all'editor con tutto com'era; da lì `Shift+F2` salva il tuo testo con un
  altro nome e lascia stare il file.

Le frecce o `Tab` passano da un pulsante all'altro e `Enter` prende quello acceso.

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

Quando il nome non dice nulla, o dice la cosa sbagliata — uno script senza estensione, un
`.txt` che in realtà è una patch, un `.inc` che in un progetto è PHP e nel successivo Pascal —
`Alt+Shift+Y` elenca i linguaggi tra cui scegliere. Scrivi qualche lettera per restringere
l'elenco e premi `Invio`. La prima riga torna a ciò che dice il nome, la seconda legge il file
come testo semplice senza colori, e la riga in vigore è marcata. La scelta viene ricordata per
quel file, dove il visualizzatore ricorda la tabella codici in cui ha letto un file, così al
prossimo `F4` non serve ripeterla (`viewer.remember = false` spegne quella memoria, per
entrambi).

In una unit Pascal o Delphi, e in un form letto come testo, una linea sottile scende lungo
ogni blocco come la disegna l'IDE di Delphi: dalla riga che lo apre — `begin`, `try`, `case`,
`repeat`, `asm`, una `class` o un `record` con un corpo, un `object` di un form — al suo `end`
o `until`, nella colonna dove comincia quella riga, una linea per ogni blocco dentro un altro.
Sono le stesse linee che disegna il visualizzatore, e qui seguono il testo mentre scrivi: apri
un blocco e la sua linea compare, chiudine uno o commenta il suo `begin` e se ne va. Una linea
passa soltanto per celle vuote, quindi non copre mai un carattere, e non fa parte del file —
nulla di lei viene copiato o salvato. Il colore è `syntax.guide` del tema, e l'interruttore
che spegne i colori spegne anche le linee.

# I numeri di riga

`Alt+N` numera le righe in un margine a sinistra, e `Alt+N` di nuovo toglie i numeri: è il
tasto di mcedit. Il margine è largo quanto il numero dell'ultima riga, e mai meno di tre
cifre; il testo, le guide dei blocchi e un blocco marcato stanno alla sua destra, e quando una
riga lunga scorre di lato i numeri restano dove sono. Sono disegnati nel `syntax.guide` del
tema, il colore delle guide dei blocchi, e come quelle non fanno parte del file: non vengono
copiati né salvati, e un clic su un numero non sposta nulla.

Il tasto vale per il file che hai davanti. **Numera le righe**, nelle impostazioni dell'editor
(`editor.line_numbers` in `fcmd.toml`), è ciò con cui si apre ogni file.

# Byte invece di righe

`F4` su un binario apre lo stesso editor sui suoi byte, e `Alt+H` passa da una parte
all'altra. Due riquadri, l'esadecimale e il testo; `Tab` si sposta fra i due e in entrambi si
scrive. `Insert` sceglie fra sovrascrivere e inserire — in un binario un byte spostato di solito
rompe il file, quindi parte in sovrascrittura.

I byte cambiati sono disegnati come cambiati fino a che non vengono scritti, con accanto
l'elenco di cosa è cambiato: posizione, byte vecchio, byte nuovo. `Ctrl+Z` ne riporta indietro
uno. `F2` scrive, in modo atomico.

`F7` cerca nei byte con la stessa finestra: come testo, come byte esadecimali — `DE AD ?? EF`,
dove `??` è un byte qualsiasi — o come espressione regolare. Quello che viene trovato è marcato
e il cursore ci va sopra; `Shift+F7` trova il successivo e `Alt+F7` il precedente, e una ricerca
che esce da un capo del file riparte dall'altro e lo dice. Un file di qualsiasi dimensione è
cercato un pezzo alla volta, mai caricato intero.

`Ctrl+F7` (o `F8`) sostituisce. La sostituzione si scrive come la ricerca — esadecimale per
esadecimale, e una riga vuota per cancellare quello che è stato trovato. *Sostituisci* si ferma
a ogni corrispondenza e chiede: sostituire questa, questa e tutte le successive, saltarla, o
fermarsi. *Tutti* fa l'intero file in una volta, e un solo `Ctrl+Z` riporta indietro tutto. Una
sostituzione della stessa lunghezza sovrascrive; una di lunghezza diversa sposta tutto quello
che segue, quindi si fa solo dopo che `Insert` è passato all'inserimento.

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

# Maiuscole e minuscole di un blocco

`Alt+C` cambia maiuscole e minuscole del blocco marcato, da un elenco di quattro:

| Riga | `Hello wORLD` diventa |
| --- | --- |
| TUTTO MAIUSCOLO | `HELLO WORLD` |
| tutto minuscolo | `hello world` |
| Iniziali Maiuscole | `Hello World` — una maiuscola all'inizio di ogni parola, minuscole dopo |
| iNVERTI | `hELLO World` — ogni maiuscola diventa minuscola e ogni minuscola maiuscola |

`Invio` applica la riga sotto il cursore, ed è un solo `Ctrl+Z`. Il blocco resta marcato, così
una seconda scelta si fa sullo stesso testo. In un blocco a colonne cambiano solo le colonne
marcate. Senza nulla di marcato cambia la **parola sotto il cursore**, o quella subito prima.
Le lettere accentate seguono le loro regole — `é` diventa `É`, e la `ß` tedesca diventa `SS`,
il che allunga il testo di una lettera.

Ognuna delle quattro è anche un comando a sé — `editor.case.upper`, `editor.case.lower`,
`editor.case.title`, `editor.case.toggle` — senza tasto finché non gliene dai uno nella mappa
dei tasti.

# Un paragrafo a una larghezza

`Alt+P` reimpagina il paragrafo in cui sta il cursore in modo che nessuna riga superi un certo
numero di colonne: è il tasto di mcedit. Chiede la larghezza, e il solo `Invio` prende
l'ultima usata, che parte da 72. Un paragrafo sono le righe attorno al cursore fino alla riga
vuota più vicina in ciascuna direzione. Le sue parole vengono rimesse con uno spazio tra
l'una e l'altra, quante ne stanno in una riga, e ogni riga prende il rientro che aveva la
prima riga del paragrafo, così un paragrafo rientrato resta rientrato. Una parola più lunga
della larghezza sta su una riga da sola; nulla viene mai tagliato.

Con un blocco marcato viene reimpaginato ogni paragrafo del blocco e le righe vuote tra l'uno
e l'altro restano. In entrambi i casi è un solo `Ctrl+Z`. Su una riga vuota senza nulla di
marcato non c'è un paragrafo, e lo dice.

# La data, e il nome del file

| Tasto | Che cosa entra al cursore |
| --- | --- |
| `Ctrl+F5` | la data e l'ora, come `2026-11-11 17:42` |
| `Alt+Shift+F` | il nome del file che stai modificando |
| `Ctrl+F` | il suo percorso completo — il tasto di FAR; per un file aperto da un server, l'indirizzo che ha lì |

Ognuno è un solo `Ctrl+Z`. Un file nuovo non ancora salvato non ha ancora un nome, e lo dice.

`Alt+F5` stampa il blocco, o tutto il testo quando non ce n'è — quello che è sullo schermo, non
quello salvato l'ultima volta.

# L'elenco dei comandi (`Alt+F9`)

La barra dei tasti ha posto per dodici tasti e l'editor ne ha molti di più. `Alt+F9` elenca
quelli descritti nelle sezioni qui sopra — cercare mentre si scrive, la parola sotto il
cursore, il blocco attraverso un comando e fuori in un file, maiuscole e minuscole, il
paragrafo, ciò che si può inserire, i numeri di riga, il linguaggio — ognuno con il tasto che
lo fa. Scrivi per restringere l'elenco, `Invio` esegue la riga, `Esc` torna indietro. `F9` da
solo resta la barra dei menu, come ovunque.

# Un blocco attraverso un comando, e file dentro e fuori

Quattro tasti su `Alt+Shift`, accanto agli altri comandi sui blocchi, collegano il testo ai
programmi e ai file che gli stanno attorno.

| Tasto | Che cosa fa |
| --- | --- |
| `Alt+Shift+P` | passa il blocco marcato attraverso un comando e mette al suo posto ciò che il comando stampa |
| `Alt+Shift+O` | inserisce al cursore ciò che un comando stampa |
| `Alt+Shift+I` | legge un file al cursore |
| `Alt+Shift+W` | scrive il blocco marcato in un file |

**`Alt+Shift+P`** chiede una riga di comando — `sort -u`, `column -t`, `jq .`, `fmt -w 72`, un
formattatore — e le dà il blocco sullo standard input. Ciò che stampa prende il posto del
blocco e resta marcato, così il comando successivo si può eseguire sul risultato. Senza nulla
di marcato passa **tutto il file**, e la domanda lo dice prima che tu risponda. È **un solo
passo**: un `Ctrl+Z` riporta ciò che c'era.

Nulla viene sostituito se il comando non è riuscito. Se finisce con un errore, la riga di
stato dice quale, con ciò che il comando stesso ha detto, e il testo resta com'era; gli avvisi
che un comando stampa accanto al risultato non finiscono nel file. Il comando gira nella
cartella del file in modifica, con la stessa shell della riga di comando, e intanto si può
continuare a lavorare: se il testo che gli è stato dato è cambiato quando risponde, la
risposta riguarda qualcosa che non c'è più e viene scartata, con una riga che lo dice. Un
blocco a colonne è rifiutato: marca righe intere.

**`Alt+Shift+O`** esegue un comando a cui non viene dato nulla e mette la sua uscita dove sta
il cursore: `date`, `ls`, `git log -3`. Il ritorno a capo con cui un programma chiude l'ultima
riga viene lasciato fuori, così una riga di uscita entra in mezzo a una riga senza spezzarla.
Un blocco eventualmente marcato non viene toccato.

**`Alt+Shift+I`** chiede un file e lo legge al cursore, come testo: nella sua tabella codici e
con i suoi fine riga, letto come lo leggerebbe `F4`, e scritto, al prossimo salvataggio, come
viene scritto il file che stai modificando. Un nome senza cartella viene cercato nella
cartella del pannello.

**`Alt+Shift+W`** chiede un nome e ci scrive il blocco marcato, con il fine riga e nella
tabella codici del file che stai modificando. Un file con quel nome viene sostituito, come lo
sostituisce `Shift+F2`; il file in modifica no, perché un suo pezzo scritto sopra di esso
lascerebbe solo quel pezzo.

Il testo va a un comando e ne torna in UTF-8, qualunque sia la tabella codici in cui il file è
salvato. Sia il file sia la destinazione del blocco devono stare su questa macchina.

# Far riscrivere un blocco

Marca un blocco e premi `Ctrl+Alt+W`: fcmd chiede **che cosa deve diventare**, a parole —
`correggi i refusi`, `traduci in inglese`, `fanne una tabella`, `rinomina x in count` — e manda
il blocco, con il nome del file e le tue parole, a un servizio AI (vedi *Chiedere a un'AI* in
*Configurazione*). Prima che parta qualcosa, un dialogo dice quanto e a quale host, con
*Annulla* acceso e `F3` per vedere esattamente che cosa parte; i segreti nel blocco vengono
mascherati come ovunque.

La risposta non finisce nel file. Si apre come **confronto**: a sinistra il blocco com'è, a
destra la versione del modello, le differenze colorate, `Alt+↓` e `Alt+↑` da un cambiamento al
successivo. Lì:

| Tasto | Cosa fa |
| --- | --- |
| `Enter` | mette la versione del modello al posto del blocco — nel buffer, non sul disco; per salvare serve ancora `F2` |
| `Esc` | lascia il blocco esattamente com'è |

Una volta inserita, la riscrittura è **un solo passo**: un unico `Ctrl+Z` la toglie tutta, e il
testo nuovo resta marcato, così puoi chiedere di nuovo o annullare e vedere la differenza.

Parte solo il testo marcato, mai il resto del file, e deve essere un blocco normale (non a
colonna). Mentre il modello scrive puoi continuare a lavorare: se quando risponde il blocco non
è più il testo che era partito — ci hai scritto dentro, l'hai smarcato, hai chiuso il file —
non viene proposto niente e niente viene sostituito; la versione del modello resta nella
cartella delle risposte (`ai:`), come ogni risposta.

`Ctrl+Alt+X` (*che cos'è questo?*) dall'editor funziona allo stesso modo: il dialogo compare
sopra l'editor, e la risposta non prende il posto dell'editor — la riga di stato dice in quale
file è stata scritta.

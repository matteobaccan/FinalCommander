# Il visualizzatore (F3)

Il visualizzatore non carica mai un file. Legge la finestra che sta mostrando, così un log da
quaranta gigabyte si apre subito e `End` è un solo posizionamento invece di un conteggio di
righe.

`F2` cambia l'andata a capo, `F4` gira la ruota dei modi di leggere il file — testo, dump dei
byte, esadecimale, documento impaginato, tabella — fermandosi solo su quelli che per questo
file hanno senso, con la barra dei tasti che dice dove cade il giro successivo; `Shift+F4` è
l'esadecimale e subito indietro.

`Maiusc+F8` è l'elenco di tutte le code page che il programma offre, un elenco in cui si
scrive: `Invio` legge il file con quella, `Ins` segna una pagina come una di quelle che usi —
prende un `*` e sale in cima — e `F4` le dà un nome tuo. `F8` poi percorre le pagine che hai
segnato, e le cinque che il programma gestisce da sé fin quando non ne segni nessuna.

`F7` apre il dialogo di ricerca:
cosa cercare, se è testo semplice, una sequenza di byte esadecimali o un'espressione regolare,
se contano le maiuscole, se contano solo le parole intere — e poi *Trova successivo* o *Trova
precedente*, perché sono la stessa domanda fatta due volte. `Shift+F7` e `Alt+F7` la ripetono
avanti e indietro senza richiedere niente. `F6` passa il file all'editor nello stesso punto.

Nel dump esadecimale le frecce decidono quanti byte tiene una riga, perché di lato in un dump
non significa nient'altro: `→` la allarga e `←` la restringe, **otto byte per volta** — 8, 16,
24, 32 e così via fino a 64 — perché otto è il gruppo fra cui il dump disegna già uno spazio,
quindi ogni pressione aggiunge un gruppo intero e le colonne restano dove l'occhio se le
aspetta. Una pressione che richiederebbe una finestra più larga di quella che hai lascia la
larghezza com'è. `Ctrl+→` e `Ctrl+←` sono un'altra cosa: spostano tutto il dump di un solo
byte, così una struttura che non comincia su un multiplo della larghezza di riga si legge in
riga invece che in diagonale.

`Grigio +` e `Grigio -` passano al file successivo e precedente del pannello senza uscire dal
visualizzatore, e `Ctrl+F10` fa raggiungere al pannello quello che stai leggendo.

Tre tasti danno la finestra al file. `Ctrl+B` toglie la barra dei tasti, `Ctrl+Shift+B` la riga
in alto con il nome e la posizione, e `Ctrl+S` mette una barra di scorrimento sul bordo destro
— ogni riga che togli è una riga di testo che guadagni, che è quello che serve a una riga lunga
o a un dump esadecimale largo. Valgono per il file che stai leggendo e niente viene scritto:
`F9 → Opzioni → Impostazioni del visualizzatore` è dove dici con cosa deve aprirsi ogni file,
insieme all'andata a capo e all'ampiezza della tabulazione.

# Markdown

Un file `.md` si apre impaginato invece che come sorgente: i segni vengono via e lo spazio che
occupavano viene usato. Un titolo è in grassetto con una riga sotto, una lista ha i punti e
l'indentazione sospesa e una lista di cose da fare ha le caselle, una citazione ha una barra
lungo il fianco sinistro, un collegamento mostra il testo e l'indirizzo, un blocco recintato è
indentato e — dove il recinto nominava un linguaggio — colorato come quel linguaggio, e una
tabella è disegnata con le colonne allineate come ha chiesto la sua riga di separazione.

`Shift+F2` passa fra il documento impaginato e il suo sorgente, e `Shift+F4` lo tiene nel giro
insieme al testo, al dump e all'esadecimale. Cercare in un documento impaginato cerca quello
che è sullo schermo, non i segni che ci stanno dietro, quindi cercare una parola in una tabella
la trova nella cella.

Questo è l'unico posto in cui il visualizzatore legge più di quello che mostra: le colonne di
una tabella non si possono ricavare dal pezzo che capita di avere sullo schermo, quindi il
documento viene impaginato tutto. Oltre i quattro megabyte resta il sorgente, e lo dice.
*Interfaccia → Markdown impaginato per essere letto* riporta la prima schermata al sorgente per
sempre.

# Le immagini

Un `.png`, un `.jpg`, un `.gif` o un `.bmp` si apre come immagine. Da lì `F4` prosegue verso il
dump e l'esadecimale per le volte in cui sei venuto per i byte — il file resta un file.

È disegnata a mezzi blocchi: una cella è alta circa il doppio di quanto è larga, quindi una
cella con dentro `▀`, la metà di sopra di un colore e quella di sotto di un altro, sono due
pixel quasi quadrati, e la finestra è una tela alta in pixel il doppio delle righe. È grossolana,
e funziona in *qualunque* terminale — ed è per questo che viene prima dei protocolli di kitty,
iTerm2 e sixel e non dopo.

Decide quello che il file *è*, non come si chiama: un `.txt` che in realtà è un PNG si apre
come immagine. Una fotografia viene rimpicciolita mantenendo la forma, e un'icona piccola resta
della sua dimensione invece di essere gonfiata in quadratoni. Quello che è trasparente lascia
vedere lo sfondo del visualizzatore.

Non si scorre — l'immagine è già grande quanto la finestra — e un file rovinato, troppo grande
o che non è un'immagine lo dice, invece di lasciarti davanti a uno schermo vuoto.

# Base64

`Alt+B` decodifica un file che è base64 e mostra quello che ne esce, tenendo nel titolo il nome
del file. Un certificato, una chiave, un allegato tirato fuori da una mail, un `data:` URI
incollato in un file: sono tutti byte scritti con sessantaquattro caratteri stampabili, e sono
tutti illeggibili finché qualcosa non lo disfa.

Quello che ne esce decide come viene disegnato: testo quando è testo, il pannello esadecimale
quando non lo è, che è il caso normale. La riga di stato dice quanti byte sono usciti, ed è da
lì che si capisce se la decodifica era quella giusta.

Si leggono tre forme: il contenuto nudo, a capo o no, con o senza riempimento; l'**armatura
PEM**, dove le righe `-----BEGIN …-----` vengono saltate e si decodifica quello che sta in
mezzo; e un **`data:` URI**, dove tutto fino a `;base64,` è un'etichetta. Vanno bene entrambi
gli alfabeti — quello standard e quello per URL che usano i token.

Un file che è solo *quasi* base64 viene rifiutato, e il messaggio dice quale byte l'ha fermato
e dove: decodificare fino al primo carattere sbagliato restituirebbe byte che non ha scritto
nessuno. Il tasto non torna indietro, perché questa è l'unica vista che *sostituisce* il file
invece di disegnarlo in un altro modo — `Esc` e riaprirlo sono la strada per tornare.

# Le tabelle

Un `.csv`, un `.tsv`, un `.jsonl` o un `.dbf` si apre come tabella invece che come le virgole,
le graffe o l'esadecimale: una testata che resta dov'è mentre le righe le scorrono sotto,
colonne allineate, un record per riga. `Shift+F3` passa fra la tabella e il sorgente — il
sorgente a volte è proprio la cosa che si era venuti a guardare — e `Shift+F4` tiene la tabella
nel giro insieme al testo, al dump e all'esadecimale. `F7` cerca quello che è sullo schermo,
quindi cercare un valore lo trova nella sua cella.

`Shift+F3` non è legato a quei nomi: fa una tabella di **qualunque** file che si legga come
tale, quindi un `.txt` di colonne separate da tabulazione o un log incolonnato si aprono come
tabella. Rifiuta solo un file che non ha nessuna tabella dentro — uno che non è testo, o uno
che non si divide in due colonne o più — e poiché lo stesso tasto torna indietro, un
tentativo sbagliato costa una pressione. Quello che si apre *da sé* è l'elenco più corto,
perché come si apre un file deve essere prevedibile.

`←` e `→` spostano la finestra di lato su una tabella più larga dello schermo, e si fermano
dove finisce l'ultima colonna invece di continuare nel vuoto. `Ctrl+←` e `Ctrl+→` saltano una
colonna intera per volta — una tabella ha un'unità per andare di lato che un muro di testo non
ha. `Home` e `End` vanno alla prima e all'ultima schermata di righe.

`Ctrl+F3` ordina le righe per la colonna dove ti trovi: una volta in salita, un'altra in
discesa, la terza torna all'ordine che hanno nel file. `Ctrl+F7` chiede che cosa deve contenere
una riga e tiene solo quelle; una risposta vuota le ridà tutte. Una colonna di numeri si ordina
come numeri, quindi 1000 viene dopo 9 e non prima, e la riga di stato dice che cosa sta facendo
la vista — `↑name`, `/tor` — così lo vedi sempre. Nessuno dei due scrive niente: cambia
l'ordine in cui le righe si leggono, il file resta com'è, e una ricerca o un segnalibro
continuano a voler dire il record. Una tabella di più di duecentomila record li rifiuta
entrambi e lo dice, perché leggere un milione di record per un tasto non è a cosa serve questa
modalità. Modificare una cella non c'è di proposito: un visualizzatore che scrive è un editor,
e quello sta in `F4`.

La riga di stato dice come è stato letto il file, quale separatore è stato *indovinato* — una
virgola, un punto e virgola, una tabulazione o una barra, quello che ha dato una tabella — e
quante righe e colonne ci sono: è così che capisci a prima vista che l'ipotesi era giusta. Un
`.tsv` non viene indovinato. Per JSON Lines le colonne sono le chiavi dei primi duecento
record, nell'ordine in cui stanno nel file; un record con una chiave comparsa più tardi la
mostra in un'ultima colonna chiamata `…` invece di perderla, e una riga che non è un oggetto
viene contata in fondo alla riga di stato invece di essere mostrata come una riga di cose senza
senso.

Un `.dbf` è diverso e migliore: una tabella dBase dice nella propria testata quanti record ha,
quanto è lungo uno e come si chiamano le colonne, quindi non si legge niente a parte la
finestra che stai guardando — una tabella da due gigabyte si apre veloce come un log, e il
limite di dimensione qui sotto non la riguarda. Le date vengono fuori come `AAAA-MM-GG`, il
vero e il falso come parole, un campo memo come quello che tiene il `.dbt` o il `.fpt` accanto
alla tabella, su una riga, e i campi numerici stanno a destra della loro colonna, perché una
colonna di numeri si legge sull'ultima cifra. Un record **cancellato** viene mostrato invece di
essere lasciato fuori — una tabella le cui righe cancellate svaniscono è una tabella il cui
conto non torna — e l'asterisco che lo dice ha una colonna stretta tutta sua davanti ai campi,
dove non si può confondere con parte di un numero.

Il testo viene fuori nella code page che la testata nomina, comprese quelle DOS: una tabella
scritta sotto DOS in Germania è in code page 437, e `temporäre` si legge `temporäre` e non
`tempor„re`.

Come un documento Markdown impaginato, una tabella di testo viene letta tutta — la larghezza di
una colonna non si può sapere dal pezzo di file che è sullo schermo — quindi oltre i sedici
megabyte il file si apre come testo e lo dice. *Interfaccia → Apri un .csv o un .jsonl come
tabella* riporta la prima schermata al sorgente per sempre. Una cella più lunga di quaranta
colonne viene tagliata con un `…`; lo scorrimento orizzontale è il modo di leggere il resto.

Cosa questo modo non è: un foglio di calcolo e uno strumento di interrogazione. Non si
modifica, non si ordina e non si filtra, perché ognuna di quelle cose vuole una risposta su
come riscrivere il file, che un visualizzatore non ha.

Il testo è colorato secondo il linguaggio che il nome del file implica, come nell'editor. Il
visualizzatore legge solo quello che mostra, quindi la colorazione comincia in cima allo
schermo: un commento aperto più su nel file non viene portato fin lì. È il prezzo di aprire
subito un log da quaranta gigabyte, e lo paga solo il visualizzatore — l'editor ha tutto il
file e lo colora esattamente.

# I binari

`F5` disassembla. x86, x86-64 e AArch64 sono decodificati nel programma stesso; su un'altra
architettura chiede a `objdump`, e lo dice invece di indovinare. Le istruzioni AArch64 sono
lunghe quattro byte ciascuna, quindi una parola che non si decodifica è una riga `(bad)` e non
quattro, e un salto è mostrato come l'indirizzo a cui arriva, non come lo scostamento scritto
nell'istruzione. `Enter` su una `call` o una
`jmp` la segue e `Backspace` torna, con i posti da cui sei saltato tenuti sullo schermo.

Su un disassemblato `F7` cerca le *istruzioni*, non i byte: `call` trova tutte le chiamate,
`mov *, eax` tutti gli spostamenti in quel registro, e `*` sta per quello che c'è fra i pezzi
del modello.

`F11` apre la struttura di un eseguibile: ELF, PE, Mach-O, `ar` e le famiglie più vecchie — NE,
LE e LX, e il TE di cui è fatto il firmware UEFI — ogni campo di testata con i
byte grezzi accanto al significato, le sezioni, gli import e gli export, e i simboli — che poi
danno il nome alle funzioni nel disassemblato. `Enter` su una posizione prende i byte che stanno
lì.

I file sorgente, i class file e il bytecode hanno un listato tutto loro dove ha senso: un
`.class` è mostrato come lo mostra `javap -c`, con la constant pool risolta nei commenti.

# Due file affiancati

`Ctrl+Shift+D` — *Confronta due file* nel menu Comandi — mette il file sotto il cursore
del pannello di sinistra accanto a quello sotto il cursore di destra e segna quello che
cambia. *Confronta per contenuto* risponde a quali di cento coppie sono diverse; questo
risponde a cosa, in una coppia, è diverso.

Ogni colonna porta i propri numeri di riga, e una riga che esiste da un solo lato ha il numero
da quel lato e niente di fronte. Dove una riga è cambiata invece di comparire o sparire, la
parte che è davvero cambiata è segnata dentro la riga, così l'occhio va al carattere che conta
invece di rileggere tutte e due.

`Alt+↓` e `Alt+↑` vanno da una differenza all'altra, che è il modo in cui si legge un diff;
l'ultima dice che è l'ultima invece di ricominciare dall'inizio. Le frecce, `PgSu`, `PgGiù`,
`Home` e `Fine` muovono per righe, `←` e `→` scorrono le due colonne insieme, e `Tab` cambia
quale delle due stai leggendo. `F12` elenca questa schermata con i visualizzatori e gli editor
aperti, `F1` apre questo capitolo, e `Esc`, `F3` o `F10` la chiude.

Tre tasti cambiano cosa conta come differenza, e ognuno riallinea subito i due file: `W` ignora
gli spazi, `C` ignora le maiuscole, `E` ignora come finiscono le righe. I fine riga partono da
quello che è stato detto l'ultima volta al dialogo del confronto, così i due sono d'accordo su
cosa vuol dire "uguale" — lo stesso checkout su Windows e su Linux è diverso in ogni riga e in
niente che conti.

Due file uguali si aprono comunque, e la schermata lo dice invece di lasciarti scorrere alla
ricerca di una differenza che non c'è.

I due file devono essere su questo disco e sotto i 16 MiB: di ognuno si legge tutto in memoria
per allinearli, quindi un file su un server o dentro un archivio lo dice col suo nome invece di
scaricarne una copia che non hai chiesto.

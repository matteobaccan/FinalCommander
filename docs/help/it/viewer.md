# Il visualizzatore (F3)

Il visualizzatore non carica mai un file. Legge la finestra che sta mostrando, così un log da
quaranta gigabyte si apre subito e `End` è un solo posizionamento invece di un conteggio di
righe.

`F2` cambia l'andata a capo, `F4` gira la ruota dei modi di leggere il file — testo, dump dei
byte, esadecimale, documento impaginato, tabella — fermandosi solo su quelli che per questo
file hanno senso, con la barra dei tasti che dice dove cade il giro successivo; `Shift+F4` è
l'esadecimale e subito indietro. `F8` cambia la code page e `F7` apre il dialogo di ricerca:
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

`F5` disassembla. x86 e x86-64 sono decodificati nel programma stesso; su un'altra
architettura chiede a `objdump`, e lo dice invece di indovinare. `Enter` su una `call` o una
`jmp` la segue e `Backspace` torna, con i posti da cui sei saltato tenuti sullo schermo.

Su un disassemblato `F7` cerca le *istruzioni*, non i byte: `call` trova tutte le chiamate,
`mov *, eax` tutti gli spostamenti in quel registro, e `*` sta per quello che c'è fra i pezzi
del modello.

`F11` apre la struttura di un eseguibile: ELF, PE, Mach-O e `ar`, ogni campo di testata con i
byte grezzi accanto al significato, le sezioni, gli import e gli export, e i simboli — che poi
danno il nome alle funzioni nel disassemblato. `Enter` su una posizione prende i byte che stanno
lì.

I file sorgente, i class file e il bytecode hanno un listato tutto loro dove ha senso: un
`.class` è mostrato come lo mostra `javap -c`, con la constant pool risolta nei commenti.

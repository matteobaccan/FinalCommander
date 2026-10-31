# I due pannelli

Ogni pannello mostra una cartella. `Tab` passa dall'uno all'altro; la maggior parte delle
operazioni va dal pannello che ha il cursore all'altro, ed è questo che rende un gestore a due
pannelli più rapido di un albero: la destinazione è già sullo schermo.

`Enter` entra in una cartella o esegue un file, `Backspace` oppure `..` risale. `Alt+F1` e
`Alt+F2` aprono il dialogo dei dischi per il pannello sinistro e destro, sopra il pannello a
cui appartiene: una tabella di tutto quello che è montato — che cosa è ciascuno (disco,
rimovibile, ottico, di rete, in memoria), come si chiama, con che filesystem è formattato,
quanto è grande e quanto ne resta. Quali di queste colonne vengano disegnate lo scegli tu, con
`F9` dentro il dialogo.

Un disco su cui l'*altro* pannello è già aperto si apre dove sta quel pannello, non in cima
al disco. Due pannelli su un disco solo è il modo in cui si lavora su un disco — un albero
da una parte, una destinazione dall'altra — e arrivare nella radice vorrebbe dire riscendere
un percorso che è già sullo schermo. Il disco su cui un pannello è già aperto continua ad
aprirsi nella radice: scegliere quello è il modo di arrivare in cima con un tasto solo. E
così `/`, che è il prefisso di qualunque percorso esista e altrimenti non vorrebbe più dire
la radice. Un pannello su un server, o che mostra una delle liste sotto la riga di
separazione, non è su un disco e non presta niente; un pannello dentro un archivio presta la
cartella in cui l'archivio sta, invece di un percorso che riaprirebbe l'archivio.

Sotto una riga di separazione, su `Ctrl+1`…`Ctrl+9`, stanno le cose che non sono filesystem:

* le **connessioni salvate** e gli **object store** scritti nella configurazione, che si
  collegano quando li scegli;
* i **processi** in esecuzione sulla macchina, come pannello — anche `Ctrl+W`, ed *Elenco
  attività* nel menu Comandi, che è quello che quel tasto vuol dire in FAR: `F3` su uno è la
  sua scheda, che è una pagina — cos'è, di chi è, cosa ha preso e speso, la riga di comando, tutto il suo
  ambiente e ogni libreria che ha caricato. Qui niente termina un processo: il tasto che
  cancella un file lo dice invece di farlo;
* i **servizi** che la macchina esegue in sottofondo, con lo stato di ciascuno: `F3` è la sua
  scheda, `F5` lo avvia, `F6` lo riavvia e `F8` lo ferma — gli ultimi due chiedono prima.
  Un'azione che il sistema rifiuta viene riferita come «serve root»; questo programma non
  chiederà root al posto tuo;
* i **socket**, in ascolto o connessi, con l'indirizzo a ciascun capo e il processo che li
  tiene, dove `F3` è la scheda di quel processo;
* i **programmi installati** sulla macchina, con versione ed editore: `F3` è la scheda, e
  scegliendone uno il suo comando di disinstallazione finisce sulla riga di comando, non
  eseguito, perché tu lo legga prima di premere `Enter`;
* un **pannello temporaneo**, che è un elenco e non una cartella: `Ctrl+Shift+Ins` ci aggiunge
  quello che è selezionato da qualunque posto, e i file dentro possono venire da una dozzina di
  cartelle diverse.

Lo stesso elenco lo può riempire un comando invece della tua mano: *Comandi → Pannello
dall'uscita di un comando* chiede una riga di comando, la esegue dove sta il pannello, e prende
ogni riga che stampa come il nome di un file. `find . -newer Makefile`, `git ls-files`,
`grep -rl TODO .` — qualunque cosa nella shell che nomini dei file ti dà un pannello di quei
file, e da lì `F5`, `F6`, `F8` e i tasti di selezione funzionano su quell'elenco come su
qualunque altro. Una riga che nomina qualcosa che non c'è viene lasciata fuori, e la riga di
stato dice quante. È l'*external panelize* di Midnight Commander, e vale la pena averlo perché
la shell sa trovare cose che nessun dialogo nostro chiederà mai.

`Ctrl+R` dentro il dialogo rilegge i dischi, per una chiavetta infilata dopo che si era aperto.
Quello su cui gira il sistema — tutto ciò che sta sotto `/proc`, `/sys`, `/dev`, `/run` — resta
fuori dalla tabella, perché niente di quello è un posto dove mettere un file.

# Guardare una cartella

`Ctrl+1`…`Ctrl+5` scelgono come vengono disegnate le righe: breve, media, completa, larga,
dettagliata. Le viste a più colonne scorrono in basso lungo una colonna e poi passano a
quella dopo, come negli originali. La vista dettagliata aggiunge, a quello che mostra la
completa, la data in cui il file è stato creato e quella in cui è stato letto l'ultima volta, e
gli attributi dopo di esse — su un pannello troppo stretto per tutto questo le date in più
vanno via per prime, così il nome resta leggibile. La vista completa tiene l'estensione in una
colonna tutta sua, contro la dimensione: i nomi lungo il bordo sinistro, le estensioni lungo il
destro. Una cartella con un punto nel nome, un file che comincia per punto e un'estensione
troppo lunga per la colonna tengono il nome intero.

`Ctrl+8` mostra a chi appartengono i file: nome, dimensione, proprietario, gruppo. I due nomi
prendono lo spazio che aveva la data, e su un pannello stretto il gruppo se ne va prima del
proprietario. Scoprirlo costa una interrogazione per file — e su Windows una chiamata che può
arrivare fino a un controller di dominio — quindi viene fatto in sottofondo e solo mentre
questa è la vista sullo schermo: le colonne si riempiono un momento dopo l'elenco, e una riga
per cui non è ancora tornato niente resta vuota. Un pannello su un server o dentro un archivio
non ha nessun proprietario da mostrare.

`Ctrl+9` mette accanto a ogni file quanti nomi ha, ed è così che cento copie che in realtà sono
un solo file smettono di sembrare cento file di spazio occupato; `Ctrl+0` è la vista completa
con le dimensioni scritte cifra per cifra — `1 234 567` dove la completa dice `1.18 M` — per
quando la differenza fra 1,2M e 1,9M è proprio la cosa che sei venuto a vedere.

Ognuna delle dieci è un elenco di colonne che puoi riscrivere: un blocco `[[column_set]]` in
`fcmd.toml` il cui `id` è la cifra della vista è quello che quella vista disegna, con le
colonne, le larghezze, gli allineamenti e i formati che scegli tu — comprese le stringhe
`Column types` di FAR. Il capitolo sulla configurazione ne dà la forma.

`Ctrl+F3`…`Ctrl+F11` ordinano: per nome, estensione, data di scrittura, dimensione, non
ordinato, data di creazione, data di lettura, descrizione e proprietario — tutti e nove i tasti
di FAR. Scegliere l'ordine già in vigore lo inverte, e `Ctrl+F12` li offre come elenco. Gli
ultimi due ordinano per qualcosa che l'elenco non porta con sé, quindi vengono recuperati come
le viste che li mostrano: il pannello ordina senza di essi e riordina quando la risposta
arriva, tenendo il cursore sul file su cui stava. Un file di cui nessuno ha detto niente si
ordina dopo tutti i file di cui qualcosa è stato detto.

Due file che la chiave di ordinamento non riesce a separare vengono risolti da
`panel.sort_chain` nella configurazione — `["size", "name"]` ordina per dimensione e risolve la
parità per nome — e il nome ha comunque l'ultima parola, così l'ordine non dipende mai
dall'ordine in cui la cartella è tornata dal sistema.

`Shift+F11` — `Gruppi` sulla barra dei tasti — accende e spegne i **gruppi di ordinamento**.
Sono fasce scritte nella configurazione come blocchi `[[sort_group]]`, ciascuna con una
maschera e un lato, e tirano certi tipi di file davanti all'elenco e ne spingono altri dietro
qualunque sia la chiave di ordinamento: i sorgenti per primi e i prodotti della compilazione
per ultimi, in una cartella che contiene entrambi. Dentro una fascia vale ancora l'ordinamento
normale, le cartelle tengono il loro posto in cima, e `reverse` inverte ogni fascia invece di
scambiare le fasce fra loro. Se non c'è nessun blocco scritto il tasto lo dice.

Non devi scriverle a mano: `F9 → Opzioni → Gruppi di ordinamento` è la stessa lista che hanno i
gruppi di colore — le fasce nell'ordine in cui tagliano l'elenco, `Ins` per una nuova accanto
alla riga su cui sei, `Canc` per toglierne una, `F4` per modificarla, `Ctrl+↑` e `Ctrl+↓` per
spostarla oltre la vicina. I due pannelli si riordinano appena accetti, e *Salva impostazioni*
scrive i blocchi nella configurazione.

`Spazio` misura quello che una cartella contiene — e con qualcosa di selezionato, tutte le
cartelle selezionate in una volta, che è come si scopre quale di cinque alberi è quello
grosso. La cifra prende poi il posto della parola `Cartella` nella colonna della dimensione, la
riga dei totali in fondo la conta, e l'ordinamento per dimensione mette quella cartella dove le
spetta. Rileggere la cartella se ne dimentica, perché la cifra riguardava la cartella com'era.

`Alt+F10` trova una cartella per nome: un elenco di tutte le cartelle che questo programma
conosce sotto la radice in cui sei, che si restringe mentre scrivi, e `Enter` ci va. Parte da
quello che l'albero ha già letto — conservato fra una sessione e l'altra — e legge il resto
dietro al dialogo mentre scrivi. Nel dialogo di copia lo stesso `Alt+F10` riempie la
destinazione.

`Ctrl+H` mostra i file nascosti. `Ctrl+P` nasconde il pannello in cui non sei, `Ctrl+F1` e
`Ctrl+F2` nascondono quello di sinistra o di destra per dare all'altro tutto lo schermo, e
`Ctrl+O` li toglie entrambi per mostrare quello che hanno stampato i comandi. Con i pannelli
spenti tutto il resto funziona ancora — la riga di comando, i menu, i tasti — e `PgSu` e
`PgGiù` girano le pagine dell'output; `Ctrl+O` riporta i pannelli.

I pannelli se ne vanno da sé mentre un comando gira, così quello che stampa si può guardare
arrivare riga per riga invece di comparire tutto insieme alla fine, e tornano quando il comando
finisce. `Ctrl+O` restituisce lo schermo prima del tempo e il comando continua dietro di essi;
*Opzioni → Interfaccia → Mostra l'output di un comando mentre gira* spegne tutta l'abitudine, e
l'output continua a riempire lo schermo mentre viene stampato, per quando premi `Ctrl+O`.

Quello che esegue i comandi è **una shell sola, tenuta viva per tutto il tempo in cui il
programma è aperto**. La riga le arriva come l'hai scritta, e questo vuol dire che quello che un
comando fa sopravvive al comando: `cd ..` sposta il pannello, un `export` è ancora impostato per
il comando dopo, una funzione che definisci resta chiamabile, e un tuo alias vuol dire quello
che vuol dire nella tua shell — non c'è una seconda shell in mezzo. Vale anche al contrario:
spostando il pannello la shell viene dietro, così il comando dopo gira dove stai guardando.
`Ctrl+C` sullo schermo dell'output manda l'interruzione a quello che ci sta girando dentro.

La shell è la tua — `$SHELL`, o quello che dice `[term] shell` — e a `bash`, `zsh`, `fish` e a
una `sh` qualunque si può chiedere dove sono. A `cmd.exe` no, quindi su Windows con `cmd` ogni
comando gira per conto suo come ha sempre fatto; niente a metà. *Opzioni → Tutti i parametri →
`term.subshell`* spegne del tutto la shell tenuta viva, e `term.follow_cwd` spegne solo
l'inseguimento.

*Sinistra* / *Destra* → *Code page dei nomi…* legge i nomi di questo pannello in una code page
scelta. Ogni pannello parte da *come li dà il sistema*, che su Linux e macOS vuol dire che i
byte vengono presi come UTF-8 — e un nome che non è UTF-8 esce pieno di caratteri di
sostituzione. Succede a un albero copiato da un Windows russo, o estratto da uno zip che non
portava la sua code page: quei byte sono davvero un nome, in CP866 o CP1251, e i punti di
domanda sono nostri e non del disco. Scegli la pagina e i nomi diventano leggibili.

Cambia solo lo schermo. Il file che ogni riga rappresenta è sempre quello che ha il sistema,
byte compresi, quindi aprirlo, copiarlo, rinominarlo e cancellarlo funzionano come prima — e la
prima riga della lista ti riporta alla risposta del sistema. Su Windows non c'è niente da
scegliere: un nome lì è già testo e non byte, e il menu lo dice. Su macOS di solito non c'è
niente da scegliere nemmeno lì — il sistema si rifiuta di creare un nome che non sia UTF-8
valido — quindi questa è soprattutto una questione Linux.

# Cosa è un file

Una cartella può tenere una riga su ciascuno dei suoi file, in un file di testo chiamato
`descript.ion` accanto a essi — l'abitudine di FAR, e di 4DOS prima di lui. `Ctrl+Z` ne scrive
una: il dialogo si apre su quello che la cartella già dice del file sotto il cursore, e una
risposta vuota porta via la riga. `Ctrl+6` mostra i nomi con le loro descrizioni, `Ctrl+7`
aggiunge la dimensione e l'orario, il pannello informativo (`Ctrl+L`) riporta per esteso quella
del file sotto il cursore, e la riga di stato del pannello la mostra accanto alla dimensione e
alla data.

Il file viaggia con la cartella, che è il punto di tutto: una cartella copiata su una chiavetta
si porta dietro le descrizioni. E una descrizione segue il suo file: `F5` ne scrive una anche
all'altro capo, `F6` la porta via con il file, una rinomina la porta sul nome nuovo, e `F8` la
porta via — una descrizione sopravvissuta al suo file finirebbe sul primo file a cui venisse
dato quel nome, anche quando i file vengono da più cartelle insieme, come capita con il
risultato di una ricerca. Tutto questo vale anche su un server e in un bucket; dentro un
archivio la riga si legge ma non si scrive, perché scriverla vorrebbe dire ricostruirlo.
*Opzioni → Impostazioni del pannello* spegne tutta l'abitudine per chi non tiene descrizioni.

Una maschera scritta su `+` o `-` prende i file e lascia stare le cartelle, che è quello che
si intende con `*.rs`; *Le maschere selezionano anche le cartelle* in *Opzioni → Impostazioni
dei pannelli* cambia le cose. *Seleziona tutto* e *inverti tutto* prendono le cartelle in ogni
caso, perché non nominano nessuna maschera.

`Ctrl+R` rilegge. `Ctrl+L` trasforma l'altro pannello in un pannello informativo — cosa
contiene il dispositivo e quanto ne resta, quanto sono grandi i suoi cluster e quanto costano a
questa cartella, quanta memoria ha la macchina, e la riga sotto il cursore — `Ctrl+Q` in
un'anteprima del file sotto il cursore — o, su una cartella, nello stesso blocco di cifre su
dispositivo e cartella che mostra il pannello informativo — `Ctrl+T` in un albero delle
cartelle.

L'anteprima è il visualizzatore stesso, senza la sua cornice. Finché il fuoco è sui file segue
soltanto il cursore; dagli il fuoco con `Tab` e i suoi tasti sono quelli del visualizzatore:
`F2` manda a capo, `F4` gira fra testo, dump, esadecimale, documento e tabella, `Shift+F4` è
l'esadecimale e ritorno, `F5` disassembla, `F6` decompila, `F7` cerca, `F8` cambia la codifica,
`F11` apre il pannello della struttura. La barra dei tasti lo dice finché ha il fuoco. Quello
che scrivi va comunque alla riga di comando. E accanto a una lista di processi, di socket o di
programmi installati l'anteprima tiene la scheda di quella riga — la stessa pagina che apre
`F3` lì, che è più lunga del pannello e scorre.

Un pannello locale si rilegge anche da sé: un file che arriva da una compilazione, da uno
scaricamento o da un altro programma compare senza che tu lo chieda, e il cursore resta sul
nome su cui stava. L'impostazione è `Aggiorna il pannello automaticamente` in
`Opzioni → Impostazioni del pannello` (`panel.auto_refresh`); `Ctrl+R` c'è ancora per un
server, un archivio, e le volte in cui la sorveglianza non ha niente da riferire.

`Ctrl+←` e `Ctrl+→` spostano la linea fra i due pannelli. `Ctrl+↑` alza il bordo inferiore dei
pannelli — diventano più corti, e le righe che cedono mostrano l'output dell'ultimo comando,
che è come si guarda scorrere una compilazione senza lasciare i pannelli; `Ctrl+↓` restituisce
le righe. Sono entrambe impostazioni, quindi `Shift+F9` le conserva.

`Ctrl++` e `Ctrl+−` chiedono al terminale un font più grande o più piccolo, e `Ctrl+Alt+0`
quello iniziale — `Ctrl+0` è la decima vista, come in FAR. Se poi succeda qualcosa è affare del
terminale e non nostro: un programma in un terminale non possiede il proprio font, può solo
chiedere. xterm e i terminali che lo imitano rispondono; sulla maggior parte degli altri (GNOME
Terminal, Konsole, Windows Terminal, alacritty, foot) quei tasti sono lo zoom dell'emulatore
stesso e non arrivano mai fino a noi — e ti danno la stessa cosa per una strada più corta.

# Cartelle a portata di mano

`Ctrl+Shift+0`…`Ctrl+Shift+9` mettono su una cifra la cartella in cui sei; `Alt+0`…`Alt+9` ti
riportano là. Dieci, come in FAR — e `Alt` invece del controllo destro di FAR perché nessun
terminale sa distinguere i due tasti controllo.

Una cifra tiene una cartella: impostarla di nuovo da un'altra parte la sposta.

`Ctrl+D` apre l'elenco completo, dove `Ins` conserva la cartella in cui sei, `Del` ne cede una
e `F4` le dà un nome, così l'elenco può dire `lavoro`, `log`, `server` invece di tre percorsi.
Quelle con una cifra vengono prima, e la cifra ti porta là con una pressione.

Una cartella su un server o in un bucket si può tenere come qualunque altra; andarci apre prima
la connessione.

# Le linguette

Un pannello può tenere più di una cartella alla volta. `Ctrl+Alt+T` apre un'altra linguetta —
sulla cartella sotto il cursore se il cursore è su una, altrimenti su quella in cui sei, che è
un duplicato ed è quello che serve prima di prendere due strade diverse dallo stesso posto.
`Ctrl+Alt+N` e `Ctrl+Alt+P` scorrono la fila, che è un anello: dopo l'ultima viene la prima.
`Ctrl+Alt+W` ne chiude una. Un pannello ha sempre una linguetta, quindi l'ultima resta.

Una linguetta ricorda tutto quello che ricorda il pannello — dov'è, come è ordinato, cosa è
selezionato, dov'è il cursore, dov'è stato — così tornare su una linguetta è tornare al lavoro
che ci avevi lasciato.

`Ctrl+Alt+M` consegna la linguetta all'altro pannello. È uno spostamento: la linguetta lascia
questo lato.

`Ctrl+Alt+K` blocca una linguetta sulla sua cartella. Una linguetta bloccata non va da nessuna
parte — non con `Enter`, non con la cronologia, non guidata dall'altro pannello — e lo dice
quando glielo chiedi, invece di non fare niente. È marcata nella fila. L'unica cosa che la
sblocca è la chiusura della connessione che le sta sotto: una linguetta non può mantenere una
promessa su una cartella che non raggiunge più.

I menu Sinistra e Destra tengono tutto questo, e anche *Dai un nome a questa linguetta*, che è
come una fila arriva a dire `src  build  log` invece di tre nomi di cartella. Un nome vuoto lo
toglie di nuovo.

La fila delle linguette è disegnata solo da un pannello che ne ha più di una: con una sola
linguetta il pannello è esattamente come è sempre stato, invece di costare una riga di file per
dire che c'è una linguetta.

Le linguette tornano quando riavvii il programma, con i loro nomi e i loro blocchi. La cartella
in cui era la shell vince comunque per la linguetta su cui atterri — è questo che significa
lanciare un programma in una cartella — a meno che quella linguetta sia bloccata. Una linguetta
che era su un server o in un bucket non viene ripristinata, perché la connessione che le serve
non è ancora aperta.

# Un terminale vero

La riga di comando sotto i pannelli esegue un comando e conserva quello che ha stampato, che è
quello che vuoi nove volte su dieci. Con qualcosa scritto sopra, `↑` e `↓` camminano avanti e
indietro per quello che è stato eseguito prima, come fa una shell; su una riga vuota le frecce
appartengono al pannello, come ogni tasto condiviso. `Alt+F8` mostra tutta la cronologia come
elenco — e così fanno `Alt+F11` per i file che hai letto o modificato e `Alt+F12` per le
cartelle in cui questo pannello è stato. Tutti e tre sono elenchi in cui si scrive — come
ogni elenco di questo programma: quello che scrivi lo restringe, la cornice dice cosa hai
scritto, `Backspace` toglie una lettera, e `Esc` pulisce il filtro prima che un secondo chiuda
l'elenco. Dove le righe hanno tasti propri, come le lettere del dialogo dei dischi, quei tasti
restano loro e il filtro è `Alt` più la lettera. `Alt+←` e `Alt+→` fanno scorrere di lato le
righe lunghe — una cronologia di percorsi profondi è più larga della sua cornice — e
`Alt+Home` le rimette a posto. `Ins` fissa la riga sotto il cursore —
una riga fissata è marcata con un `*`, e né il limite di dimensione né la durata possono
spingerla fuori — `Maiusc+Canc` toglie quella riga dalla cronologia per sempre, e `Canc`
svuota l'elenco di tutto ciò che non hai fissato, dopo averlo chiesto. Una riga che nessuno ha
fissato e nessuno usa da novanta giorni se ne va da sola; il numero sta in *Opzioni →
Impostazioni di sistema*.

`Ctrl+Invio` mette sulla riga il nome sotto il cursore, `Ctrl+Maiusc+Invio` quello dell'altro
pannello, `Ctrl+[` e `Ctrl+]` i percorsi dei due pannelli, `Ctrl+Maiusc+[` e `Ctrl+Maiusc+]`
quelli del pannello sinistro e destro qualunque abbia il fuoco, `Ctrl+F` il nome completo e
`Ctrl+Alt+F` il nome completo nella forma che un'altra macchina potrebbe usare — un percorso
su questo disco non ne ha una, e il tasto lo dice.

Mentre scrivi, la riga ti offre quello che potrebbe diventare: i nomi della cartella che il
pannello sta mostrando, le righe che hai già eseguito, e le variabili d'ambiente. `↑` e `↓`
scorrono l'elenco, `Tab` prende quello sotto il cursore, `Esc` lo chiude — e `Enter` continua a
eseguire quello che hai scritto finché non sei entrato nell'elenco, così l'elenco non è mai
d'intralcio. `Ctrl+Spazio` lo chiede, e sveglia le sorgenti che aspettano di essere chieste:
gli eseguibili sul tuo `PATH` sono quella che lo fa, perché raccoglierli vuol dire leggere ogni
cartella del `PATH`.
*Opzioni → Interfaccia* è dove ciascuna delle quattro sorgenti si accende, si spegne, o si
mette a rispondere solo quando le viene chiesto.

`Shift` con una freccia marca il testo, `Ctrl+Shift` con una freccia marca una parola per
volta, e `Shift+Home`/`Shift+Fine` arrivano a un capo o all'altro. Quello che è marcato è
disegnato con i colori scambiati; `Ctrl+Ins` copia poi *quello* invece di tutta la riga, e
scrivere o incollare lo sostituisce. Una freccia senza `Shift` lascia cadere il segno.

La decima volta è `vim`, `top`, `ssh` o qualunque altra cosa disegni uno schermo intero e
chieda al sistema quanto è grande: quelle cose vogliono un *terminale*, e `Ctrl+Alt+E` è un
terminale.

È una schermata come il visualizzatore e l'editor. `Ctrl+Alt+E` di nuovo torna ai pannelli e
lascia il programma in esecuzione; `F12` lo elenca fra tutto quello che è aperto e lo riporta
davanti; premere `Ctrl+Alt+E` dai pannelli riporta davanti un terminale parcheggiato invece di
avviarne un secondo. Un secondo si ottiene con lo stesso tasto dopo aver parcheggiato il primo,
e la cosa è voluta: un tasto che avvia un processo in silenzio ogni volta che lo premi
lascerebbe dietro una dozzina di shell in un giorno.

Dentro, ogni tasto appartiene al programma — `F3` significa quello che `F3` significa per quello
che sta girando, `Tab` completa, `Ctrl+C` interrompe. Esattamente un tasto è nostro, ed è
quello che ti ha fatto entrare. Non c'è barra dei tasti per la stessa ragione: una barra che
offrisse i nostri tasti lì dentro ti direbbe una cosa falsa.

La shell è quella che la tua sessione usa già (`$SHELL`, o `ComSpec` su Windows), così gli alias
e il prompt sono quelli che conosci. `[term] shell` in `fcmd.toml` ne nomina un'altra, come
riga di comando: `bash -l`, o un percorso con spazi fra apici. Parte nella cartella che il
pannello attivo sta guardando, quando quella è una cartella vera — un pannello dentro un
archivio o su un server non ne ha, e la shell parte da dove è partito il programma.

Quando il programma finisce, la sua ultima schermata resta su per essere letta, e il tasto
successivo la chiude.

# Scegliere i file

`Insert` seleziona la riga e va avanti. `Grigio +` e `Grigio -` selezionano e deselezionano per
maschera; `Grigio *` inverte. `Ctrl+A` di una maschera, `Alt+lettere` per trovare un nome
mentre scrivi.

Quello che è selezionato è quello su cui agisce l'operazione successiva. Senza niente
selezionato è la riga sotto il cursore — così `F5` su un file copia quel file, e nessuno deve
selezionarlo prima.

# Il mouse

Spento fino a che non lo chiedi, in `F9 → Opzioni → Impostazioni dell'interfaccia`, e la ragione
vale la pena di saperla: mentre un terminale riferisce il mouse smette di agirci lui, quindi
selezionare il testo col mouse e incollare col tasto centrale smettono di funzionare. Quelli
sono strumenti del tuo terminale e questo programma non può sostituirli, quindi non te li
toglie senza che tu lo chieda.

Con il mouse acceso: un clic porta il cursore sulla riga e rende attivo quel pannello, un
doppio clic è `Enter` su di essa, il tasto destro seleziona una riga, un trascinamento da una
riga seleziona tutte le righe che tocca — seleziona soltanto, quindi è il tasto destro che ne
toglie una — un trascinamento sulla cornice dove i due pannelli si incontrano sposta il confine
fra loro, un trascinamento lasciato sull'*altro* pannello apre il dialogo di copia con quella
destinazione già scritta (non copia niente da sé: il dialogo e il suo `Esc` sono al loro
posto), e la rotella scorre quello che sta sotto il puntatore — tre righe a scatto, senza prendere il fuoco, così puoi leggere
l'altro pannello senza lasciare questo. Quante righe fa uno scatto lo decidi tu, e
separatamente per i pannelli, il visualizzatore e l'editor: in *Tutti i parametri* ci sono
`ui.wheel_lines`, `ui.wheel_lines_viewer` e `ui.wheel_lines_editor`. La rotella orizzontale, se
il mouse o il trackpad ce l'ha, scorre una riga lunga nel visualizzatore o nell'editor; sopra
un pannello non fa niente, perché le colonne di un pannello sono una disposizione e non una
finestra su qualcosa di più largo. La barra dei tasti mostra sempre tutti e dodici i tasti
funzione, con quelli che una schermata non usa lasciati vuoti, così `F5` è la quinta casella
dovunque ti trovi. È cliccabile (tieni `Shift` e clicca per `Shift+F5`), e così sono la barra
dei menu e l'elenco di un dialogo. Un clic fuori da un dialogo aperto non fa assolutamente
niente.

# Copiare e incollare testo

`Ctrl+Ins` copia i nomi di quello che è selezionato — uno per riga — e `Alt+Shift+Ins` li copia
con tutto il percorso. Con qualcosa sulla riga di comando, `Ctrl+Ins` copia invece quella riga:
la stessa regola di ogni altro tasto condiviso, che appartiene al pannello solo mentre la riga è
vuota.

La copia la fa il tuo terminale, attraverso una richiesta chiamata `OSC 52`, ed è per questo che
funziona anche su `ssh`: gli appunti che si riempiono sono quelli della macchina davanti a cui
sei seduto, non quelli della macchina su cui gira il programma. Un terminale che non capisce la
richiesta la ignora, e non può rispondere né sì né no, quindi quello che ti viene detto è cosa è
stato copiato — non che gli appunti l'abbiano preso.
`F9 → Opzioni → Impostazioni dell'interfaccia` la spegne.

`Alt+Ins` copia invece un pezzo di **schermo**, che è la risposta per tutto quello che il
programma ha disegnato e che non ha un file dietro: la scheda di un processo, una pagina
dell'aiuto, un dump esadecimale, la frase con cui è finito un confronto. Tiene lo schermo come
è e ci mette un cursore: le frecce muovono, `Maiusc` insieme a loro segna un rettangolo da
dove il cursore era già, `Ctrl+A` segna tutto lo schermo, `Invio` o `Ctrl+Ins` copia e chiude,
ed `Esc` chiude senza copiare niente. Senza niente di segnato prende la riga su cui sta il
cursore. Gli spazi in coda a ogni riga se ne vanno, perché un rettangolo intorno a una riga
corta è fatto quasi solo di spazi.

Incollare è un tasto del tuo terminale, non nostro — `Shift+Ins` o `Ctrl+Shift+V` nella maggior
parte di essi — e il testo arriva qui in un pezzo unico invece che come una sequenza di tasti.
L'editor lo prende tutto, ritorni a capo compresi; la riga di comando, una riga di dialogo e una
domanda sono una riga ciascuna, quindi prendono la prima riga e dicono quante altre ce n'erano.
Un comando copiato da una pagina web si porta dietro il ritorno a capo, ed eseguirlo prima di
averlo letto è esattamente quello che quella regola evita.

# I dialoghi

Un dialogo è un elenco di righe in una cornice, con una riga di pulsanti sotto. Una casella
quadra `[x]` è un sì o no a sé stante; una rotonda `(•)` è una risposta di un gruppo, e
scegliendola le altre del gruppo si spengono; una riga che finisce con `↓` contiene un elenco
di valori che `Spazio` e le frecce percorrono; `[?]` è una casella chiesta di più file che non
sono d'accordo, e lasciata com'è non ne cambia nessuno.

`↑` e `↓` si spostano fra le righe. `Tab` percorre tutto il dialogo e torna all'inizio — le
righe, poi i pulsanti, poi la prima riga — e `Shift+Tab` lo percorre a rovescio, così il tasto
che esce dalla prima riga è la via d'uscita. `Invio` risponde al dialogo da dove si trova il
cursore, `F10` è `OK` da qualunque punto, `Esc` butta via tutto.

Una riga che nomina un percorso si completa con `Tab`, come in una shell, e `Ctrl+↓` apre quel
che è già stato scritto in quella riga. `Alt+F10` su una riga così apre da dove scegliere:
l'albero delle cartelle per una riga che vuole una cartella — la destinazione di una copia, la
cartella di un'estrazione — e la cartella stessa per una riga che vuole un file, come il file
di chiave di una connessione. In quell'elenco `Invio` su una cartella ci entra e `Invio` su un
file è la risposta; `Esc` lascia la riga com'era. `Ctrl+F5` passa le frecce al dialogo stesso, per quando
copre l'unica cosa che serve leggere; `Invio` ed `Esc` lo fermano soltanto.

La riga in basso dello schermo diventa quella del dialogo finché è aperto: i tasti dei pannelli
non sono sullo schermo, e descriverli sarebbe descrivere qualcosa che non c'è. Quasi tutte le
caselle sono vuote, perché a un dialogo si risponde con `Invio` ed `Esc` — `F1` è questa
pagina, `F10` è `OK`, e `Ctrl` mostra il tasto che lo sposta. `F1` lascia il dialogo in attesa dietro l'aiuto, con tutto quello
che è stato scritto dentro ancora al suo posto.

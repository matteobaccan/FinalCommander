# Key bar labels: short, they must fit in a few columns.
key-help = Nápov
key-menu = UživMn
key-view = Zobraz
key-edit = Uprav
key-copy = Kopír.
key-print = Tisk
key-move = PřjPřs
key-mkdir = VytAdr
key-delete = Smazat
key-pulldown = KonfMn
key-quit = Konec
# The key bar while a modifier is held: FAR's own words, six columns wide.
key-add = Přidat
key-extract = Rozbal
key-arccmd = ArcPří
key-edit-new = Uprav…
key-rename = Přejme
key-group = Skupin
key-selup = VybPrv
key-left = Levý
key-right = Pravý
key-sort-name = Název
key-sort-ext = Přípon
key-sort-write = Změna
key-sort-size = Veliko
key-sort-unsort = Neřadi
key-sort-create = Vytvoř
key-sort-access = Přístu
key-sort = Třídit
key-mklink = VytLnk
key-find = Hledat
key-history = Histor
key-folders = AdrsHs
key-plugins = Moduly

# Panel chrome
list-up = Nahoru
list-folder = Adresář
col-name = Název
col-ext = Příp
col-size = Velikost
col-allocated = Alokováno
col-modified = Změněno
col-created = Vytvořeno
col-accessed = Přístup
col-attr = Atr
col-owner = Vlastník
col-group = Skupina
col-descr = Popis
col-links = Lnk
col-git = Git
descr-title = Popis { $name }
descr-set = { $name }: popis zapsán
descr-cleared = { $name }: popis odstraněn
descr-not-here = popisy se uchovávají vedle adresáře na tomto počítači
col-date-time = Datum      Čas
panel-list-count = položek v seznamu: { $count }
panel-empty = (prázdné)
panel-loading = načítání…

# Prompts
prompt-select-mask = Označit soubory podle masky:
prompt-deselect-mask = Odznačit soubory podle masky:
prompt-goto = Přejít do adresáře:
prompt-filter = Maska souborů nebo část názvu:
prompt-search = Rychlé hledání:

# Dialogs
dialog-error = Chyba
dialog-help-hint = Esc se vrací k obsahu · F7 hledá ve všech kapitolách · F1 zavírá · ↑↓ PgUp PgDn posouvají
dialog-help-index = FinalCommander { $version } — Nápověda
dialog-help-hint-index = Enter otevře kapitolu · F7 hledá ve všech kapitolách · Esc zavírá
help-search-prompt = Hledat v nápovědě
help-search-none = Žádný řádek nápovědy neobsahuje „{ $query }“
help-search-title = „{ $query }“ v nápovědě: řádků — { $count }
output-search-prompt = Hledat ve výstupu
output-search-empty = Zatím nebylo nic vypsáno
output-search-found = Řádek { $line } z { $lines }
output-search-wrapped = Řádek { $line } z { $lines }, znovu od nejnovějšího
output-search-none = Žádný řádek výstupu neobsahuje „{ $query }“

# Status messages
msg-not-implemented = { $command } v tomto sestavení ještě není implementováno
msg-selected = označeno položek: { $count }
msg-deselected = odznačeno položek: { $count }
msg-mask-invalid = neplatná maska: { $error }
msg-theme = motiv: { $name }
msg-font-bigger = písmo: terminál požádán o větší
msg-font-smaller = písmo: terminál požádán o menší
msg-font-reset = písmo: terminál požádán o původní
msg-sort = řazení: { $key }{ $reverse }
msg-sort-groups-on = skupiny řazení zapnuty ({ $count })
msg-sort-groups-off = skupiny řazení vypnuty ({ $count })
msg-sort-groups-none = nejsou definovány žádné skupiny řazení: viz [[sort_group]] v konfiguraci
msg-view = zobrazení: { $mode }
msg-hidden-on = skryté soubory zobrazeny
msg-hidden-off = skryté soubory skryty
msg-no-history = žádná další historie
msg-tab-last = panel má vždy jednu kartu
msg-tab-end = karta už je na tomto konci řady
tabs-title = Karty
msg-tab-moved = karta přesunuta do druhého panelu
msg-tab-locked = tato karta je uzamčena ke svému adresáři
msg-tab-locked-on = karta uzamčena ke svému adresáři
msg-tab-locked-off = karta odemčena
msg-bookmark-set = zkratka { $digit }: { $path }
msg-bookmark-unset = zkratka { $digit } není nastavena
msg-bookmark-added = { $path } uloženo
msg-bookmark-removed = { $path } odebráno
msg-bookmark-known = tento adresář už v seznamu je
msg-dirsize = { $path }: { $bytes }, souborů: { $files }
msg-dirsize-many = měření adresářů: { $count }…
usage-title = Využití disku: { $path } · { $size } · Tab: celý strom jako mapa
usage-not-here = využití disku se měří u adresáře na disku, ne uvnitř archivu nebo seznamu nalezených souborů
msg-dirsize-no-folder = není tu žádný adresář k měření

# Jobs: prompts, confirmations, progress, conflicts
prompt-copy-to = Kopírovat { $what } do:
prompt-move-to = Přejmenovat nebo přesunout { $what } do:
prompt-mkdir = Vytvořit adresář:
prompt-tab-name = Název této karty:
prompt-bookmark-name = Název tohoto adresáře:
what-selected = označené položky ({ $count })
confirm-delete-title = Smazat
confirm-delete-body =
    { $count ->
        [one] Smazat { $name } ({ $bytes })?
        [few] Smazat { $count } položky ({ $bytes })?
       *[other] Smazat { $count } položek ({ $bytes })?
    }
prompt-link = Vytvořit link na { $name } jako:
link-created = { $name } vytvořen
confirm-wipe-title = Vymazat
confirm-wipe-body =
    { $count ->
        [one] Přepsat a smazat { $name } ({ $bytes })?
        [few] Přepsat a smazat { $count } položky ({ $bytes })?
       *[other] Přepsat a smazat { $count } položek ({ $bytes })?
    }
confirm-wipe-honest = Bajty se přepíší jednou a poté se soubor smaže. Na SSD, v souborovém systému s kopírováním při zápisu nebo se snímky mohou starší kopie přežít — a název zůstává v adresáři i v žurnálu.
job-copy = Kopírování
job-move = Přesouvání
job-delete = Mazání
job-wipe = Vymazávání
job-trash = Přesouvání do koše
job-replace = Nahrazování
job-pack = Balení
job-rebuild = Zápis archivu

# ── the attributes dialog (SPEC 10 §3.8) ───────────────────────────────────────────────
attr-title = Atributy souboru
attr-count = označených položek: { $count }
attr-now = nyní { $text } ({ $octal })
attr-owner-read = Vlastník smí číst
attr-owner-write = Vlastník smí zapisovat
attr-owner-exec = Vlastník smí spouštět
attr-group-read = Skupina smí číst
attr-group-write = Skupina smí zapisovat
attr-group-exec = Skupina smí spouštět
attr-other-read = Všichni smějí číst
attr-other-write = Všichni smějí zapisovat
attr-other-exec = Všichni smějí spouštět
attr-setuid = Spouštět jako vlastník (setuid)
attr-setgid = Spouštět jako skupina (setgid)
attr-sticky = Mazat smí jen vlastník (sticky)
attr-readonly = Pouze pro čtení
attr-hidden = Skrytý
attr-system = Systémový
attr-archive = Archivní
attr-temporary = Dočasný
attr-offline = Offline
attr-indexed = Indexer do něj smí nahlížet
attr-compressed = komprimovaný
attr-encrypted = šifrovaný
attr-sparse = řídký
attr-reparse = reparse point
attr-also = také { $what }
attr-mtime = Změněno
attr-atime = Poslední čtení
attr-btime = Vytvořeno
attr-ctime = Změna záznamu
attr-btime-readonly = (čas vytvoření lze nastavit jen ve Windows)
attr-utc = Časy v UTC
attr-owner = Vlastník (uživatel:skupina)
attr-owner-is = vlastník { $who }
btn-attr-original = Původní
btn-attr-current = Aktuální
btn-attr-blank = Vyprázdnit
attr-recursive = A vše uvnitř adresářů
attr-bad-time = `{ $text }` není datum a čas (YYYY-MM-DD HH:MM:SS)
attr-bad-owner = žádný účet ani skupina se tu nejmenuje `{ $text }`
attr-in-archive = tato změna zapíše archiv znovu, ostatní položky zůstanou, jak jsou
job-attrs = Nastavování atributů

# ── the copy dialog (SPEC 03 §4) ───────────────────────────────────────────────────────
copy-title = Kopírovat
copy-title-move = Přejmenovat nebo přesunout
copy-target = Do
copy-conflict = Pokud už soubor existuje
copy-conflict-ask = zeptat se
copy-conflict-overwrite = přepsat
copy-conflict-newer = přepsat, je-li zdroj novější
copy-conflict-skip = přeskočit
copy-conflict-rename = dát kopii volný název
copy-conflict-append = připojit na konec
copy-only-newer = Jen soubory novější než ty existující
copy-filter = Jen názvy odpovídající masce
copy-follow-links = Kopírovat to, na co link ukazuje, ne link
copy-keep-times = Zachovat časy
copy-keep-mode = Zachovat oprávnění
copy-verify = Každý soubor přečíst zpět a porovnat
copy-several = cílů: { $count }, jeden po druhém

# ── the terminal (SPEC 06 §4) ──────────────────────────────────────────────────────────
term-started = terminál, spuštěn { $shell }
term-hint = Ctrl+Alt+E: zpět k panelům
term-ended = skončil — libovolná klávesa toto zavře
job-mkdir = Vytváření adresáře
job-planning = plánování…
job-counters = souborů: { $files } / { $filestotal } · { $bytes } / { $bytestotal }
job-rates = { $speed }/s · uplynulo { $elapsed } · zbývá { $eta }
job-rates-no-eta = { $speed }/s · uplynulo { $elapsed }
job-hint = Esc ruší
job-hint-queue = Esc ruší · B na pozadí
job-queued = { $kind } čeká ve frontě, nedokončených úloh před ní: { $ahead } · Alt+Shift+F5 je vypíše
job-background = { $kind } pokračuje na pozadí · Alt+Shift+F5 vypíše úlohy
job-asks = { $kind } čeká na odpověď · Alt+Shift+F5
job-paused = { $kind }: pozastaveno
job-resumed = { $kind }: pokračuje
job-retry-nothing = Není co opakovat
job-still-waiting = Tato úloha čeká, až přijde na řadu
jobs-title = Úlohy (nedokončených: { $active })
jobs-hint = Enter zobrazit · Space pauza · Del zastavit · F5 znovu · Ctrl+↑↓ pořadí · Esc zavřít
jobs-quit-title = Úlohy stále běží
jobs-quit-body = Tyto úlohy se zastaví, pokud program skončí teď:
job-state-running = běží
job-state-paused = pozastaveno
job-state-asking = ptá se
job-state-queued = čeká
job-state-held = zadrženo
job-state-done = hotovo
job-state-failed = chyby
job-state-cancelled = zrušeno
job-done = { $kind }: souborů: { $files }, { $bytes }
job-done-renamed = { $kind }: souborů přesunutých bez kopírování: { $files }
job-done-skipped = , přeskočeno: { $skipped }
job-done-verified = , ověřeno: { $verified }
job-done-unverified = , nešlo tu přečíst zpět: { $unverified }
job-done-errors = , chyb: { $errors }
job-cancelled = { $kind }: zrušeno
job-errors-title = Chyby
conflict-title = Cíl už existuje
conflict-dst = cíl    { $path }   { $bytes }
conflict-src = zdroj  { $path }   { $bytes }
conflict-newer = zdroj je novější
conflict-older = cíl je novější
btn-overwrite = Přepsat
btn-overwrite-all = Vše
btn-skip = Přeskočit
btn-skip-all = Přeskočit vše
btn-retry = Znovu
btn-move = Přesunout
btn-delete = Smazat
btn-wipe = Vymazat
btn-copy = Kopírovat
btn-discard = Zahodit
btn-kill = Ukončit
btn-print = Tisk
btn-quit = Konec
btn-play = Přehrát
btn-run = Spustit
btn-accept = Přijmout
btn-cancel = Storno
msg-nothing-to-do = nic není označeno
cmd-complete-nothing = není čím to doplnit
complete-several = názvů, které tak začínají: { $count }
form-moving = šipky posouvají dialog · Enter nebo Esc ho tam nechá
form-moved = dialog zůstává tam, kam jste ho umístili
msg-trash-refused = Koš to nepřijal — Shift+Del smaže rovnou.

# Checksums (SPEC 03 §9)
sum-title = Kontrolní součty
sum-algorithm = Hash
sum-one-file = Jeden seznam pro všechny
sum-name = Název
sum-working = { $name }: { $done } z { $total }
sum-verifying = kontrola podle { $name }
sum-stopped = zastaveno
sum-written = { $name } zapsán
sum-no-name = seznam potřebuje název
sum-local-only = kontrolní součet lze spočítat jen pro soubor na tomto počítači
print-title = Tisk
print-body = Vytisknout {$what}?
print-sending = odesílání na tiskárnu…
print-sent = odesláno na tiskárnu
print-failed = tisk se nezdařil ({$line}): {$error}
print-local-only = vytisknout lze jen soubor na tomto počítači
print-no-command = chybí příkaz tisku: nastavte [print] command nebo nainstalujte lp či lpr
sum-all-match = všech { $count } souhlasí
sum-verify-title = Kontrolní součty
sum-verify-summary = souhlasí: { $ok }, nesouhlasí: { $bad }
sum-differs = nesouhlasí
sum-missing = chybí
sum-unreadable = nelze přečíst

# Synchronising two directories (SPEC 03 §10)
sync-title = Synchronizace
sync-between = { $left }  ↔  { $right }
sync-recursive = Také vše v podadresářích
sync-both-ways = Oběma směry, vyhrává novější
sync-delete-extra = Nabízet odstranění toho, co druhá strana nemá
sync-by-content = Porovnávat bajty, ne velikost a čas
sync-filter = Jen názvy odpovídající masce
sync-walking = procházení { $at } — zatím { $pairs }
sync-planned = ke kopírování: { $copies } ({ $bytes }), k odstranění: { $deletes }
sync-hint = Space další odpověď · ← → směr · = nechat · F2 provést · Esc storno
sync-hint-walking = Esc zastaví procházení
sync-nothing-to-do = každý řádek říká nechat vše, jak je
sync-local-only = synchronizace umí číst a zapisovat jen na tomto počítači
sync-same-directory = oba panely jsou ve stejném adresáři

# What to run for a kind of file (SPEC 06 §5)
assoc-none = nic tu neříká, jak otevřít { $name }
usermenu-empty = Uživatelské menu zatím není: vytvořte ho v { $path }
usermenu-last-gone = řádek zvolený minule už v tomto menu není
usermenu-bad-condition = Podmínce nerozumím — { $problem }
assoc-desktop = Předat pracovní ploše
assoc-open-with = Otevřít { $name } pomocí
assoc-no-pager = ani asociace, ani $PAGER neříká, čím to číst
assoc-no-editor = ani asociace, ani $VISUAL neříká, čím to upravovat

# Viewer (F3)
key-wrap = Zalom
key-hex = Hex
key-text = Text
key-dump = Výpis
key-doc = Dok
key-search = Hledat
key-code = ZnSady
key-follow = Sledov
viewer-prompt-search = Hledat ({ $mode }):
viewer-prompt-goto = Jít na pozici nebo NN% :
viewer-not-found = `{ $query }` nenalezeno
viewer-follow-on = sledování konce souboru
viewer-follow-off = režim sledování vypnut
viewer-codepage = kódová stránka: { $name }
viewer-wrap-mode = { $mode }
viewer-no-undo = žádná dřívější pozice
viewer-empty = (prázdný soubor)
viewer-not-markdown = není to soubor Markdown, nebo je příliš velký na vysázení
viewer-not-table = není to tabulkový soubor, nebo je příliš velký na načtení celý
viewer-base64-done = Dekódováno: { $bytes }
viewer-not-base64 = Tento soubor není base64: { $why }
viewer-status = { $mode } { $codepage }  { $offset } / { $len }  { $percent }%

key-disasm = Disasm
viewer-disasm-failed = { $name } nelze disassemblovat: { $error }
viewer-decompile-running = { $tool } pracuje na { $name }…
viewer-decompile-none = nic tu neumí dekompilovat { $format }; nainstalujte nástroj a přidejte pravidlo [[decompile]] do fcmd.toml
arc-needs-program = tento archiv čte `{ $program }`, který na tomto počítači není nainstalován
viewer-decompile-failed = { $tool }: { $error }
viewer-blame-derived = git blame se týká samotného souboru: nejprve se k němu vraťte
viewer-derived = { $producer } pro { $name }

# Editor (F4)
key-save = Uložit
key-replace = Nahraď
editor-prompt-goto = Jít na řádek nebo řádek:sloupec :
editor-prompt-save-as = Uložit jako:
editor-saved = { $name }: uloženo, bajtů: { $bytes }
editor-nothing-to-save = { $name } nemá žádné neuložené změny
editor-not-found = `{ $query }` nenalezeno
editor-replaced = nahrazení: { $count }
editor-read-only = { $name } je pouze pro čtení
editor-readonly-on = pouze pro čtení: úpravy se odmítají
editor-readonly-off = režim pouze pro čtení vypnut
editor-no-undo = už není co vrátit zpět
editor-no-redo = není co zopakovat
editor-discard-title = Neuložené změny
editor-discard-body = { $name } má neuložené změny. Odejít a přijít o ně?
editor-changed-title = Změněno na disku
editor-changed-body = { $name } se od otevření na disku změnil. Uložit přes něj?
editor-reloaded = { $name } znovu načten z disku
editor-failed = { $name }: { $error }

# netbox (SPEC 04 §6)
net-sites-title = Připojení
net-sites-hint = Enter připojit · n URL · Ins uložit · F4 upravit · d odpojit · Esc zavřít
net-no-sites = Zatím žádné servery. Stiskněte `n` a zadejte URL, nebo přidejte bloky [[site]] do konfigurace.
net-not-a-stored-session = tento řádek je otevřené připojení, ne uložená relace
net-site-forgotten = { $name } už není uloženo
net-prompt-url = Připojit k:
net-prompt-password = Heslo pro { $user }@{ $host }:
net-prompt-passphrase = Heslo ke klíči { $user }@{ $host }:
net-connecting = připojování k { $host }…
net-connected = připojeno k { $host }
net-disconnected = { $host } odpojen
net-not-connected = { $host } není připojen
net-local-panel = tento panel je na místním souborovém systému
net-nothing-pending = žádné připojení na to nečeká
net-hostkey-title = Neznámý klíč hostitele
net-hostkey-body = { $host } není v known_hosts. Jeho otisk je:
net-hostkey-question = Zaznamenat ho a připojit se?
net-hostkey-changed-title = Klíč hostitele se ZMĚNIL
net-hostkey-changed-body = { $host } odpověděl klíčem, který není ten zaznamenaný.
net-hostkey-changed-where = Zaznamenaný je v { $path }, řádek { $line }.
net-hostkey-changed-what = Přeinstalovaný server vypadá přesně jako někdo uprostřed. Nic se tu nepřijímá: otevřete soubor, přečtěte si řádek a odstraňte ho sami, pokud víte, proč se klíč změnil.
btn-open-file = Otevřít soubor
net-fetching = stahování { $name }…
net-viewing-copy = { $name }: místní kopie stažená pro prohlížeč
net-too-big = { $name } je větší než { $limit }: zvyšte net.fetch_limit_mb, aby šel stáhnout
net-editing = { $name }: upravuje se místní kopie; uložení ji zapíše zpět
net-writing-back = zápis { $name } zpět…
net-written-back = { $name }: zapsáno zpět, bajtů: { $bytes }
net-write-back-failed = zápis zpět se nezdařil: { $error } — vaše úpravy jsou v { $local }
net-changed-title = Změněno na serveru
net-changed-body = { $name } se od stažení na serveru změnil. Přepsat ho?
confirm-delete-permanent = Toto maže natrvalo: pro tyto položky žádný koš není.
confirm-trash-title = Přesunout do koše
confirm-trash-body =
    { $count ->
        [one] Přesunout { $name } ({ $bytes }) do koše?
        [few] Přesunout { $count } položky ({ $bytes }) do koše?
       *[other] Přesunout { $count } položek ({ $bytes }) do koše?
    }
confirm-trash-where = Putují tam, kde pracovní plocha uchovává smazané soubory, a lze je odtud vrátit.

# ---------------------------------------------------------------- menu bar (SPEC 10)
menu-left = &Levý
menu-right = &Pravý
menu-files = &Soubory
menu-commands = Pří&kazy
menu-options = &Nastavení
menu-brief = &Stručný
menu-medium = S&třední
menu-full = &Plný
menu-detailed = Detai&lní
menu-owners = Vlastníci so&uborů
menu-links = Souborové lin&ky
menu-alt-full = Plný, velikosti v &celku
menu-descriptions = P&opisky
menu-long-descriptions = &Dlouhé popisky
menu-wide = Š&iroký
menu-info-panel = Panel In&formace
menu-quick-view = Rychlý n&áhled
menu-tree-panel = Panel St&rom
menu-tab-new = &Nová karta
menu-tab-close = Zavřít kartu
menu-tab-next = Dal&ší karta
menu-tab-prev = Př&edchozí karta
menu-tab-move = Přesunout kartu do dru&hého panelu
menu-tab-lock = Uzamknout kartu k &jejímu adresáři
menu-tab-name = Pojmenovat tuto kartu
menu-tab-list = Všechny karty
menu-sort-modes = Módy &řazení
menu-hidden = Skryté a systémové soubory
menu-filter = Filtr sou&borů
menu-filter-sets = Pojmenované filtry
menu-panel-onoff = Panel &zapnout/vypnout
menu-reread = Obno&vit
menu-change-drive = Z&měnit jednotku
menu-view = &Zobrazit
menu-edit = &Editovat
menu-edit-new = Editovat nový sou&bor
menu-copy = &Kopírovat
menu-rename = &Přejmenovat/Přesunout
menu-link = Vytvořit li&nk
menu-multirename = Přejmenovat podle vz&oru…
menu-multirename-undo = Vrátit poslední hromadné přejmenování
menu-mkdir = &Vytvořit adresář
menu-delete = &Smazat
menu-wipe = Vy&mazat
menu-checksum = Kontrolní so&učty…
menu-checksum-verify = Zkontrolovat podle seznamu
menu-split = Rozdělit soubor…
menu-combine = Spojit soubory
menu-delete-permanent = Smazat natrva&lo
menu-archive-add = Přidat do &archivu
menu-archive-extract = &Rozbalit soubory
menu-archive-commands = Příkazy arc&hivu
menu-archive-test = Otestovat archiv
menu-attributes = A&tributy souboru
menu-apply-command = Aplikovat př&íkaz
menu-replace-in-files = Nahradit v souborech…
menu-select-group = Označit skup&inu
menu-select-ext = Označit podle této přípony
menu-unselect-ext = Odznačit podle této přípony
menu-unselect-group = O&dznačit skupinu
menu-invert-selection = Invertovat výběr
menu-select-all = Označit vše
menu-restore-selection = Obnovit výběr
menu-dirsize = Velikost adresáře
menu-find-file = H&ledat soubor
menu-history = &Historie příkazů
menu-view-history = Historie &zobrazení a úprav
menu-folders-history = Historie &adresářů
menu-usage = Využití &disku
menu-jobs = &Úlohy
menu-branch = Zobrazení větve
menu-bookmarks = Adresářové zkratky
menu-swap-panels = Prohodit panel&y
menu-sync-panel = &Synchronizovat druhý panel
menu-panels-onoff = &Panely zap/vyp
menu-compare-folders = Po&rovnat adresáře
menu-sync-folders = Synchro&nizovat adresáře
menu-compare-contents = Porovnat podle o&bsahu
menu-panelize = Výstup příkazu do panelu
menu-view-filtered = Zobrazit přes příkaz
menu-learn-keys = Naučit klávesy
menu-tunnel = Otevřít &tunel…
menu-tunnels = Tun&ely…
tunnel-title = Tunel: [místní:]hostitel:port
tunnel-list-title = Tunely
tunnel-none = žádný tunel není otevřen
tunnel-open = tunel otevřen: { $what }
tunnel-closed = tunel zavřen
tunnel-failed = { $what }: { $why }
tunnel-needs-connection = tunel vede přes připojení: nejprve ho otevřete pomocí Alt+N
menu-connections = Připojení
menu-disconnect = Odpojit
menu-user-menu = Uživatelské &menu
menu-associations = Asocia&ce souborů
menu-folder-shortcuts = Adresářové z&kratky
menu-plugin-commands = Příkazy mod&ulů
menu-screens-list = Seznam obrazovek
menu-macro-record = Nahrát makro
menu-macro-list = Makra
menu-task-list = Seznam úl&oh
menu-system-settings = Nastavení s&ystému
menu-panel-settings = Nastavení &panelů
menu-interface-settings = Nastavení ro&zhraní
menu-languages = Nastavení &jazyka
menu-themes = &Motivy
menu-confirmations = P&otvrzování
menu-viewer-settings = Nastavení p&rohlížeče
menu-editor-settings = Nastavení &editoru
menu-codepages = Kó&dové stránky
menu-sort-groups = Skup&iny řazení
menu-mask-groups = Ma&sky souborů
menu-settings-all = Všechna n&astavení
menu-highlighting = Z&výrazňování souborů
menu-plugins-config = Nastavení modu&lů
menu-help = Nápověda
menu-save-setup = &Uložit nastavení
menu-quit = &Konec
opt-natural-sort = Přirozené pořadí čísel
opt-allow-reverse = Povolit obrácené řazení
opt-select-folders = Masky označují i adresáře
cmd-no-network-name = Cesta na tomto disku nemá síťový název
opt-trash = Používat systémový Koš
opt-system-copy = Používat systémovou rutinu kopírování
opt-save-history = Uchovávat historii mezi relacemi
opt-history-lifetime = Životnost historie, ve dnech
opt-forever = navždy
opt-fetch-limit = Limit velikosti vzdálených souborů
opt-keepalive = SSH keep-alive
opt-off = vypnuto
opt-show-hidden = Zobrazovat skryté a systémové soubory
opt-column-titles = Zobrazovat nadpisy sloupců
opt-status-line = Zobrazovat stavový řádek
opt-scrollbar = Zobrazovat posuvník
opt-free-space = Zobrazovat volné místo
opt-auto-refresh = Obnovovat panel automaticky
opt-dirs-first = Nejdříve adresáře
opt-selected-first = Nejdříve označené soubory
opt-reverse = Obrácené pořadí
opt-view = Zobrazení panelu
opt-sort = Řadit podle
opt-quick-search = Rychlé hledání
opt-quick-alt = s Alt
opt-quick-plain = jen písmeny
opt-key-bar = Zobrazovat zkratkové klávesy
opt-status-bar = Zobrazovat stavovou lištu
opt-syntax = Obarvovat rozpoznané jazyky
opt-markdown = Markdown vysázený ke čtení
opt-table = Otevírat .csv a .jsonl jako tabulku
opt-keymap = Rozložení kláves
opt-clock = Hodiny v rohu
opt-mouse = Podpora myši
opt-clipboard = Nechat terminál přenášet schránku (OSC 52)
opt-command-screen = Zobrazovat výstup příkazu, dokud běží
opt-menu-always = Vždy zobrazovat lištu menu
opt-save-exit = Ukládat nastavení při ukončení
opt-confirm-delete = Mazání
opt-confirm-overwrite = Přepsání
opt-confirm-exit = Ukončení
opt-confirm-editor = Opuštění editoru s neuloženými změnami
opt-applied = nastavení použito
opt-saved = uloženo do { $path }
opt-set-title = { $key }
opt-set = { $key } = { $value }
opt-not-a-toggle = Není to nastavení ano/ne: Enter umožní zadat hodnotu
opt-settings-hint = ↑↓ výběr · psaním zúžit · Enter nebo F4 hodnota · Space přepnout · Del výchozí · Esc zavřít
opt-language-set = jazyk: { $name }
sort-name = Název
sort-ext = Přípona
sort-size = Velikost
sort-mtime = Čas zápisu
sort-btime = Čas vytvoření
sort-atime = Čas přístupu
sort-descr = Popis
sort-owner = Vlastník
sort-git = Stav Git
key-sort-descr = Popis
key-sort-owner = Vlastn
sort-none = Neřadit
sort-set = řazení: { $name }
btn-ok = OK
pick-hint = ↑↓ výběr · psaním filtrovat · Enter použít · Esc zavřít
pick-empty = (není z čeho vybírat)
# ------------------------------------------------- the three histories (SPEC 06 §6)
hist-pinned = { $line } je připnuto · teď už ho nic nevytlačí
hist-unpinned = { $line } už není připnuto
hist-is-pinned = { $line } je připnuto · Ins ho nejprve odepne
hist-clear-body = Vyprázdnit tuto historii od všeho, co není připnuto?
hist-cleared = v historii zůstalo jen to, co je připnuto
history-hint = ↑↓ výběr · Enter použít · Ins připnout · Shift+Del jeden · Del vše · Esc zavřít


# ------------------------------------------------- session editor (SPEC 10 §3.5)
site-new = Nové připojení
site-edit = Úprava připojení
site-name = Název
site-host = Hostitel
site-port = Port
site-user = Uživatel
link-title = Vytvořit link
link-name = Link
link-kind = Druh
link-kind-relative = symbolický, relativní — přežije přesun stromu
link-kind-absolute = symbolický, absolutní — přežije přesun linku
link-kind-hard = pevný — druhý název téhož souboru
menu-colors = &Barvy…
palette-title = Barvy motivu { $theme }
palette-hint = Enter barva · F2 uložit · Esc zavřít
palette-ask = Barva pro { $key }
palette-bad-color = { $value } není barva (název, 0–255 nebo #rrggbb)
palette-saved = barvy byly zapsány do motivu { $theme }
palette-unsaved = barvy se změnily jen na obrazovce — F2 je zapíše
opt-viewer-status-line = Zobrazovat horní řádek (název a pozice)
opt-viewer-scrollbar = Zobrazovat posuvník
opt-viewer-key-bar = Zobrazovat zkratkové klávesy
opt-viewer-wrap = Dlouhé řádky
opt-viewer-wrap-off = nezalamovat
opt-viewer-wrap-char = zalamovat na okraji
opt-viewer-wrap-word = zalamovat po slovech
opt-viewer-tab-width = Velikost tabulátoru
opt-viewer-remember = Pamatovat si pozici a kódovou stránku každého souboru
opt-editor-tabs = Tabulátory
opt-editor-tabs-keep = nechat, jak jsou
opt-editor-tabs-typed = rozvíjet psané
opt-editor-tabs-all = rozvinout všechny při otevření souboru
opt-editor-tab-width = Velikost tabulátoru
opt-editor-auto-indent = Automatické odsazování
opt-editor-whitespace = Zobrazovat mezery a tabulátory
opt-editor-cursor-after = Kurzor se postaví za text nalezený hledáním
opt-editor-beyond-eol = Kurzor smí za konec řádku
opt-editor-trim-trailing = Při ukládání odstraňovat koncové mezery
editor-not-a-character = tato klávesa nemá znak, který by šel vložit
paste-one-line =
    { $count ->
        [one] vložen jen první řádek · jeden další řádek vynechán
        [few] vložen jen první řádek · { $count } další řádky vynechány
       *[other] vložen jen první řádek · { $count } dalších řádků vynecháno
    }
clip-names =
    { $count ->
        [one] jeden název ve schránce
        [few] { $count } názvy ve schránce
       *[other] { $count } názvů ve schránce
    }
clip-paths =
    { $count ->
        [one] jedna cesta ve schránce
        [few] { $count } cesty ve schránce
       *[other] { $count } cest ve schránce
    }
clip-line = příkazový řádek je ve schránce
clip-selection = označený text je ve schránce
clip-off = schránka je vypnuta — `ui.clipboard` v nastavení
history-gone = { $name } už neexistuje
hl-new = Nová skupina barev
hl-edit = Skupina barev
# ------------------------------------------------- the diff screen (SPEC 05 §3)
diff-title = Rozdíly
menu-diff = Zobrazit rozd&íly souborů
diff-needs-two = Postavte kurzor na soubor v každém panelu
diff-not-here = { $name } není soubor na tomto disku
diff-too-big = { $name } je větší než { $most }
diff-same = Oba soubory jsou stejné
diff-where = blok { $at } z { $count } · řádků: -{ $removed } +{ $added }
diff-no-next = Poslední rozdíl
diff-no-previous = První rozdíl
diff-hint = Alt+↑↓ bloky · Tab strana · F5 vzít · F2 zapsat · U sjednocený · Shift+F2 patch · W C E ignorovat · Esc
diff-hint-git = Alt+↑↓ bloky · S přidat blok do indexu · Tab strana · F5 vzít · F2 zapsat · Esc zavřít
diff-hint-git-staged = Alt+↑↓ bloky · S odebrat blok z indexu · Tab strana · F5 vzít · F2 zapsat · Esc zavřít
diff-stage-edited = porovnání má nezapsaná sloučení: nejprve F2, potom S
diff-merged-left = Levý soubor ten rozdíl převzal
diff-merged-right = Pravý soubor ten rozdíl převzal
diff-no-hunk-here = Nahoře na obrazovce žádný rozdíl není — Alt+↓ ho tam posune
diff-nothing-to-write = Nic nebylo sloučeno
diff-written = zapsáno: { $count }
diff-unwritten = Sloučení nebylo zapsáno — F2 ho zapíše, další Esc ho zahodí
diff-ignore-space-on = Mezery se ignorují
diff-ignore-space-off = Mezery se znovu počítají
diff-ignore-case-on = Velikost písmen se ignoruje
diff-ignore-case-off = Velikost písmen se znovu počítá
diff-ignore-eol-on = Konce řádků se ignorují
diff-ignore-eol-off = Konce řádků se znovu počítají

grab-hint = ↑↓←→ pohyb · Shift+↑↓←→ označit · Ctrl+A vše · Enter kopírovat · Esc zavřít
grab-copied = zkopírováno řádků obrazovky: { $rows }

# ------------------------------------------------- the code pages (SPEC 05 §1.12)
cp-title = Kódové stránky
cp-name-title = Jak tuto kódovou stránku pojmenovat
cp-set = Čte se jako { $name }
menu-panel-names = Kódová stránka názvů…
names-title = Číst názvy jako
names-system = Tak, jak je dává systém
names-set = Názvy se čtou jako { $name }
names-system-set = Názvy tak, jak je dává systém
names-not-here = V tomto systému název nejsou bajty, takže ho nelze číst jinak
cp-marked = { $name } je teď mezi těmi, které prochází F8
cp-unmarked = { $name } už není mezi těmi, které prochází F8
cp-hint = ↑↓ výběr · Enter číst tak · Ins označit pro F8 · F4 přejmenovat · Esc zavřít

# ------------------------------------------------- the named mask groups (SPEC 02 §5b)
mg-title = Skupina masek
mg-name = Název
mg-mask = Masky
mg-hint = Ins přidá skupinu, Del ji odebere, F4 upraví · `·` označuje tři vestavěné
mg-saved = <{ $name }> platí; „Uložit nastavení“ ji zapíše
mg-removed = <{ $name }> odebrána
mg-bad = <{ $name }>: { $problem }
mg-needs-name = název bez mezer, čárek a lomených závorek
mg-needs-mask = skupina, která nic nezastupuje, ničemu neodpovídá
mg-built-in = <{ $name }> je jedna ze tří vestavěných · F4 jí dá jiné masky

sg-title = Skupina řazení
sg-name = Název
sg-mask = Masky
sg-place = Kde
sg-place-front = před ostatními
sg-place-back = za ostatními
sg-hint = Ins přidá skupinu, Del ji odebere, F4 upraví, Ctrl+↑↓ ji posune
sg-saved = Skupina platí; „Uložit nastavení“ ji zapíše
sg-removed = { $name } odebrána
sg-bad = { $name }: { $problem }
sg-needs-mask = skupina bez masky by nic neobsahovala
hl-name = Název
hl-mask = Maska nebo masky
hl-kind = Platí pro
hl-kind-both = soubory i adresáře
hl-kind-file = jen soubory
hl-kind-dir = jen adresáře
hl-size = Velikost (1M-10M, -4K, 1G-)
hl-age = Stáří ve dnech (<=1, >=30, 7-30)
hl-date-field = Stáří se počítá podle
hl-attrs = Atributy (h r x s b, - znamená „nesmí mít“)
hl-fg = Barva textu (#d7875f, red, file.archive)
hl-bg = Pozadí, zapisuje se stejně
hl-hint = ↑↓ výběr · Enter/F4 upravit · Ins nová · Del odebrat · Ctrl+↑↓ posunout · Esc zavřít
hl-saved = skupina platí · Nastavení → Uložit nastavení ji zapíše do souboru
hl-removed = { $name } odebrána · Nastavení → Uložit nastavení to zapíše do souboru
hl-bad = { $name }: { $problem }
filter-off = Bez filtru: všechny názvy
filter-hint = ↑↓ výběr · Enter použít · F4 upravit · Ins nový · Del odebrat · Esc zavřít
filter-new = Nový filtr
filter-edit = Filtr
filter-name-hint = s názvem zůstane v menu filtrů; bez názvu se použije jednou
filter-applied = filtr zapnut: { $name }
filter-cleared = filtr vypnut
filter-saved = { $name } platí v panelu a je v menu filtrů · Nastavení → Uložit nastavení ho zapíše do souboru
filter-removed = { $name } odebrán · Nastavení → Uložit nastavení to zapíše do souboru
editor-no-block = nic není označeno
editor-unique = odstraněno opakovaných řádků: { $count }
editor-no-bracket = není tu závorka, nebo k ní nic nepatří
editor-tabs-expanded = tabulátory rozvinuty na řádcích: { $lines } — uložení to zapíše
site-protocol = Protokol
site-protocol-sftp = SFTP (přes SSH)
site-protocol-ftp = FTP
site-protocol-ftps = FTP s TLS
site-protocol-dav = WebDAV
site-protocol-davs = WebDAV přes HTTPS
site-auth = Ověření
site-auth-agent = agent SSH
site-auth-key = soubor s klíčem
site-auth-ask = zeptat se na heslo
site-key-file = Soubor s klíčem
site-start-dir = Vzdálený adresář
site-group = Skupina
site-keepalive = Keep-alive (s)
site-compress = Komprese
site-page-session = Relace
site-page-environment = Prostředí
site-page-directories = Adresáře
site-page-sftp = SFTP
site-password-note = Heslo se neukládá: program se na něj zeptá při připojování.
site-env-eol = Překládat konce řádků (zatím ne)
site-env-utf8 = Vynutit názvy v UTF-8 (zatím ne)
site-jump = Průchozí hostitel
site-jump-note = Připojení vede přes tohoto hostitele: host, user@host nebo user@host:port. Prázdné znamená přímo na server.
site-sftp-ciphers = Zvolit šifry (zatím ne)
btn-connect = Připojit
site-needs-host = připojení potřebuje hostitele
site-needs-key = zvolte soubor s klíčem nebo jiný způsob ověření
site-bad-port = `{ $port }` není číslo
site-stored = připojení zapamatováno: { $name } (Shift+F9 ho uloží)
# ------------------------------------------------- structure pane (SPEC 05 §1.8)
key-struct = Strukt
key-markdown = MD
key-table = Tabulk
key-filter = Filtr
key-decompile = Dekomp
key-next = Další
key-prev = Předch
key-codepages = ZnSady
key-goto = Jít na
key-to-panel = Panel
key-blame = Blame
viewer-structure = Struktura
viewer-structure-hint = ↑↓ řádek · Enter jít na · Tab obsah · Shift+F11 zavřít
viewer-structure-hint-tab = Tab sem přenese klávesy
viewer-no-structure = není co dekódovat: panel struktury tento formát nezná
viewer-no-target = tento řádek neukazuje nikam do souboru
viewer-form-opened = Formulář jako text, jak ho zobrazuje Delphi. F4 se vrací k bajtům.
viewer-metadata-opened = Celý text, vysázený ke čtení. F4 se vrací k bajtům.
viewer-went-to = přechod na { $offset }
viewer-in-panel = panel stojí na { $name }
viewer-not-in-panel = { $name } v tomto panelu není
viewer-no-more-files = v panelu už nejsou další soubory
editor-unsaved-for-viewer = nejprve uložte: prohlížeč čte soubor z disku
# ------------------------------------------------- command line (SPEC 06 §7)
cmd-running = běží: { $command }
prompt-panelize = Příkaz, jehož výstup jmenuje soubory
prompt-viewer-filter = Příkaz, jehož výstup číst místo souboru
learn-title = Které klávesy tento terminál posílá
learn-hint = Mačkejte klávesy. Rozsvícená buňka znamená, že klávesa dorazila; dvakrát Esc ukončí.
learn-last = poslední: { $key } — { $command }
learn-unbound = k ničemu nepřiřazena
recode-title = Zapsat znovu jako
recode-what = souborů: { $count }
recode-from = Čím jsou teď
recode-to = Zapsat je jako
recode-auto = zjistit pro každý soubor
recode-eol = Konce řádků
recode-eol-keep = nechat být
recode-working = nový zápis souborů: { $count }
recode-done = zapsáno: { $changed }, už v požadovaném stavu: { $same }, odmítnuto: { $failed }
menu-recode = Zapsat znovu jako…
panelize-title = { $command }
panelize-done = souborů: { $count }, přeskočených řádků: { $dropped }
panelize-nothing = { $command }: žádný řádek nejmenoval existující soubor
cmd-interrupted = příkazu bylo posláno přerušení
cmd-done = { $command } dokončen
cmd-failed = { $command } skončil s kódem { $code }
cmd-busy = nějaký příkaz už běží
shell-not-kept = { $shell } neoznačil výzvu včas: každý příkaz nyní běží samostatně, jako dříve
prompt-apply = Příkaz, který se spustí pro každý ze souborů ({ $count }):
cmd-apply-no-ask = příkaz, který se ptá na hodnotu, nelze aplikovat soubor po souboru
cmd-apply-progress = { $command }  ({ $done } z { $total })
cmd-apply-stopped = zastaveno po { $done } z { $total }: { $command } selhal
cmd-not-remote = příkaz potřebuje místní adresář; tento panel je na serveru
cmd-no-history = historie příkazů je prázdná
cmd-no-previous = není kam se vrátit
cmd-no-such-dir = { $path }: takový adresář neexistuje
cmd-no-such-path = { $path }: takový soubor ani adresář neexistuje
cmd-no-selection = nic není označeno
cmd-condition-false = nespuštěno: { $command }
cmd-screen-cleared = obrazovka vymazána
cmd-stack-empty = zásobník adresářů je prázdný
cmd-stack-cleared = zásobník adresářů vymazán
cmd-pushed = adresářů v zásobníku: { $count }
cmd-environment = prostředí
cmd-set = { $name } = { $value }
cmd-not-set = { $name } není nastaveno
user-screen-title = Výstup — řádků: { $lines }, od řádku { $at }
user-screen-empty = (zatím nic neběželo)
viewer-no-branch = tento řádek nemá adresu, kterou by šlo následovat
viewer-no-back = není k čemu se vracet

# ------------------------------------------------- find files (SPEC 06 §1)
folders-title = Hledat adresář
folders-reading = Načítání adresářů…
folders-read = adresářů: { $count }
folders-capped = adresářů: { $count }, po limit
folders-local-only = seznam adresářů je z tohoto počítače; server má svou historii a seznam zkratek
find-title = Hledat soubor
find-mask = Maska nebo masky souborů
find-size = Velikost (100M, 1M-1G, -4K)
find-text = Obsahující text
find-case = Rozlišovat velikost písmen
find-words = Jen celá slova
find-hex = Text výše je šestnáctkový vzor
find-not = Soubory, které ho NEOBSAHUJÍ
find-scope = Hledat v
find-scope-panel = tomto adresáři
find-scope-tree = tomto adresáři a níže
find-scope-selection = označených adresářích
find-scope-root = od kořene této jednotky
find-scope-other = druhém panelu a níže
find-links = Následovat symbolické linky
find-dirs = Adresáře také ve výsledcích
find-archives = Hledat v archivech
find-results = Nalezeno { $count } · prohlédnuto souborů: { $files }, adresářů: { $dirs }
find-done = nalezeno { $count }, prohlédnuto souborů: { $files } · { $elapsed }
find-none = nic nenalezeno
find-stopped = zastaveno po prohlédnutí souborů: { $files }
find-hint = ↑↓ výběr · Enter jít na · F3 zobrazit · F4 upravit · F5 do panelu · Esc zastavit
find-hint-done = ↑↓ výběr · Enter jít na · F3 zobrazit · F4 upravit · F5 do panelu · Esc zavřít
find-panelised = výsledků v panelu: { $count }
find-needs-mask = je potřeba maska: `*` znamená každý soubor
find-bad-size = `{ $text }` není velikost — { $error }
# ------------------------------------------------- replace across files
replace-title = Nahradit v souborech
replace-needle = Co hledat
replace-with = Nahradit čím
replace-case = Rozlišovat velikost písmen
replace-words = Jen celá slova
replace-pattern = Text výše je regulární výraz
replace-hex = Obojí jsou šestnáctkové bajty
replace-scope = označených souborů: { $count }
replace-needs-needle = není co hledat
replace-confirm-title = Nahradit
replace-confirm-body = Změnilo by se výskytů: { $count }, v souborech: { $files }.
replace-nothing = není co nahradit: žádný soubor by se nezměnil
replace-done = nahrazeno výskytů: { $count }, v souborech: { $files }

# ── archives (SPEC 04 §4) ───────────────────────────────────────────────────────────────
arc-writing-back = Zápis { $name } zpět do archivu
arc-move-out = chcete-li něco z archivu vyjmout, zkopírujte to klávesou F5
arc-add-files-only = po jednom se přidávají jen soubory; Shift+F1 přidá celý adresář

rename-title = Hromadné přejmenování
rename-name = Název
rename-ext = Přípona
rename-search = Hledat
rename-replace = Nahradit čím
rename-regex = Regulární výraz ($1 v náhradě)
rename-case = Písmena
rename-case-same = jak jsou
rename-case-lower = malá písmena
rename-case-upper = VELKÁ PÍSMENA
rename-case-title = Každé Slovo Velkým
rename-case-sentence = Jen první písmeno velké
rename-counter = Čítač: začátek, krok, číslice
rename-fields = [N] název  [E] přípona  [N1-3] část  [C] čítač  [d] [t] datum, čas  [P] adresář
rename-not-in-archive = více souborů se přejmenovává v adresáři, ne uvnitř archivu: tam F6 přejmenuje jednu položku
rename-bad-counter = čítač jsou tři celá čísla: začátek, krok, číslice; { $counter } jimi není
rename-cannot = nelze přejmenovat: { $why }
rename-problem-empty = nezbyl žádný název
rename-problem-invalid = takový název soubor mít nemůže
rename-problem-twice = tento název dostane i jiný soubor
rename-problem-exists = soubor s tímto názvem už existuje
rename-more = … a ještě { $count }
rename-blocked =
    { $count ->
        [one] jeden soubor nemůže dostat nový název: dokud to nepůjde, nic se nepřejmenuje
        [few] { $count } soubory nemohou dostat nové názvy: dokud to nepůjde, nic se nepřejmenuje
       *[other] { $count } souborů nemůže dostat nové názvy: dokud to nepůjde, nic se nepřejmenuje
    }
rename-summary = mění se { $count } z { $total }
rename-nothing = žádný název se nemění
rename-nothing-to-undo = není žádné hromadné přejmenování, které by šlo vrátit
rename-done =
    { $count ->
        [one] přejmenován jeden soubor · vrátíte to z menu Soubory
        [few] přejmenovány { $count } soubory · vrátíte to z menu Soubory
       *[other] přejmenováno { $count } souborů · vrátíte to z menu Soubory
    }
rename-undone =
    { $count ->
        [one] jeden název vrácen
        [few] { $count } názvy vráceny
       *[other] { $count } názvů vráceno
    }
pack-title = Přidat do archivu
pack-name = Archiv
pack-format = Formát
pack-level = Komprese
pack-solid = Pevné bloky (7z: menší, ale jeden soubor se čte pomaleji)
pack-delete-after = Po archivaci soubory smazat
pack-level-normal = normální
pack-level-fast = rychlá
pack-level-best = nejlepší
pack-scope = označených souborů: { $count }
pack-needs-name = archiv potřebuje název
pack-format-read-only = { $format } se tu jen čte, nezapisuje: zvolte jiný formát nebo jiný název
pack-single-one-file = { $format } pojme jeden soubor: vyberte jeden, nebo zvolte { $tar } pro adresář či více souborů
unpack-title = Rozbalit z archivu
unpack-to = Rozbalit do
unpack-scope = z { $name }
unpack-not-an-archive = není to archiv, který umíme číst

# ── the special panel kinds (SPEC 02 §7) ────────────────────────────────────────────────
panel-quick-view = Rychlý náhled
panel-info = Informace
panel-tree = Strom
compare-done = liší se: { $left } vlevo, { $right } vpravo
cmp-subdirs = Vstupovat do podadresářů
cmp-depth = Jak hluboko jít (0 = až dolů)
cmp-trees-title = Co se liší
cmp-trees-working = porovnávání stromů: souborů { $seen }, přečteno { $read }
cmp-trees-done = liší se: { $differ }, jen vlevo: { $left }, jen vpravo: { $right }, celkem párů: { $pairs }
cmp-trees-same = oba stromy jsou stejné (párů: { $pairs }, zkontrolováno: { $same })
menu-duplicates = Hledat duplikáty
dup-title = Duplicitní soubory
dup-working = hledání duplikátů: souborů { $seen }, přečteno { $hashed }
dup-done = skupin kopií: { $groups }, souborů: { $files }, promarněno { $wasted }
dup-none = nic tu není kopií něčeho jiného (souborů: { $seen })
dup-stopped = zastaveno po prohlédnutí souborů: { $seen }
dup-partial = zastaveno předčasně
cmp-title = Porovnat
cmp-case = Párovat názvy včetně velikosti písmen
cmp-size = Jiná velikost je rozdíl
cmp-time = Jiný čas je rozdíl
cmp-seconds = Časy lišící se o tolik sekund jsou tentýž okamžik
cmp-contents = Číst bajty
cmp-when-time = Číst je jen tam, kde se časy liší
cmp-fragment = Číst jen tolik prvních KiB (0 = vše)
cmp-ignore-eol = Samotné konce řádků nejsou rozdíl
cmp-selected = Jen to, co je označeno
cmp-stop = Zastavit u prvního rozdílu
cmp-newer = Označit jen novější kopii
menu-compare-setup = Porovnat s &volbami
compare-contents-eol-ignored = liší se jen konci řádků, ignorováno: { $count }
compare-contents-trusted = považováno za stejné bez čtení: { $count }
compare-contents-working = Porovnávání obsahu: { $done } z { $total }
compare-contents-done = liší se párů: { $differ } z { $pairs }
compare-contents-same = všechny páry ({ $pairs }) jsou totožné
compare-contents-none = žádný soubor nemá na obou stranách stejný název
compare-contents-local = porovnání podle obsahu potřebuje oba panely na místním souborovém systému
compare-contents-eol = jen konce řádků: { $count }
compare-contents-only = jen na jedné straně: { $count }
compare-contents-unread = nešlo přečíst: { $count }
compare-contents-stopped = porovnávání zastaveno
quick-view-directory = { $name } je adresář — F3 ho změří
quick-view-not-here = bajty tu nejsou: F3 je nejprve přinese
info-path = Cesta
info-kind = Druh
info-kind-local = adresář na tomto počítači
info-kind-remote = na { $host }
info-kind-archive = uvnitř { $name }
info-kind-list = seznam souborů
info-device = Zařízení
info-folder = Adresář
info-device-value = { $total }, volných { $free }, clustery { $cluster }
info-slack = Ztráty v clusterech
info-filesystem = Souborový systém
info-mount = Připojeno
info-git = Repozitář
info-git-clean = shodné s indexem
info-git-differing = { $count } z { $tracked } se liší od indexu
info-git-uncounted = nespočítáno
info-git-merge = probíhá slučování
info-git-rebase = probíhá rebase
info-git-cherry-pick = probíhá cherry-pick
info-git-revert = probíhá revert
info-git-bisect = probíhá bisect
info-memory = Paměť
info-memory-value = volných { $free } z { $total }
info-memory-total = { $total }
info-contents = Obsahuje
info-contents-value = souborů: { $files }, adresářů: { $dirs }, { $bytes }
info-selected = Označeno
info-selected-value = souborů: { $count }, { $bytes }
info-cursor = Kurzor
info-size = Velikost
info-modified = Změněno
info-description = Popis
key-mark = Označ
key-asm = Asm
key-block = Blok
key-screens = Obraz.
key-site-edit = UprRel
key-site-new = NováRel
key-site-forget = SmazRel
key-ai-edit = Uprav
key-ai-new = Nová
key-ai-remove = Odebrat
key-ai-forget = Zapomen
key-ai-threads = Histor
hex-needs-a-file = tento buffer nemá soubor, ze kterého by šly číst bajty
hex-save-first = nejprve uložte: dva editory nemohou držet neuložené změny jeden druhého

# ── the hex editor's block operations (SPEC 05 §2.6) ────────────────────────────────────
hex-block-title = Blok
hex-op-write = Zapsat { $size } do souboru
hex-op-read = Načíst soubor na pozici { $at }
hex-op-insert = Vložit nulové bajty na pozici { $at }
hex-op-copy = Zkopírovat bajty odjinud na { $at }
hex-op-move = Přesunout bajty odjinud na { $at }
hex-op-fill = Vyplnit { $size } bajtem
hex-op-xor = Provést výlučné NEBO nad { $size } s klíčem
hex-op-delete = Smazat { $size } — z { $was } bude { $becomes }
hex-write-title = Zapsat blok do
hex-read-title = Který soubor načíst
hex-fill-title = Kterým bajtem vyplnit (hex)
hex-xor-title = Výlučné NEBO s kterým klíčem (hex)
hex-insert-title = Kolik nulových bajtů
hex-copy-title = Odkud kopírovat — pozice a délka (hex)
hex-move-title = Odkud přesunout — pozice a délka (hex)
hex-not-hex = to není řada šestnáctkových číslic
hex-not-a-count = to není počet bajtů
hex-not-a-range = to není pozice a délka v hex
hex-range-outside = tento rozsah leží mimo soubor ({ $size })
hex-move-into-itself = blok nelze přesunout dovnitř sebe sama
hex-filled = vyplněno { $size }
hex-xored = nad { $size } provedeno výlučné NEBO — to není šifrování
hex-written = { $size } zapsáno do { $name }
hex-read-done = { $size } načteno z { $name }
hex-inserted = vloženo nul: { $size }
hex-deleted = smazáno { $size }
hex-device = { $name }: zařízení o velikosti { $size }, otevřeno jen pro čtení
asm-title = Asemblovat na { $at } ({ $arch })
asm-no-arch = { $arch } není architektura, pro kterou umíme asemblovat
asm-written = { $text } — { $bytes }
asm-written-longer = { $text } — { $bytes } (o { $grew } bajtů delší)
screens-title = Obrazovky
screens-panels = Panely
quit-title = Ukončení FinalCommander
quit-body = Ukončit?
screens-quit-title = Neuložené změny
screens-quit-body = Tyto obrazovky obsahují změny, které nikdo nezapsal:
macro-recording = Nahrávání makra · Ctrl+. ho zastaví
macro-empty = nic nebylo nahráno
macro-dropped = nahrávka byla zahozena
macro-key-title = Která klávesa
macro-key-body = Nahráno kroků: { $steps }. Stiskněte klávesu, na kterou má makro reagovat, nebo Esc pro jeho zahození.
macro-describe = K čemu je makro na { $key }
macro-saved = makro na { $key } uloženo, kroků: { $steps }
macro-none = zatím nejsou nahrána žádná makra · Ctrl+. jedno nahraje
macro-list-title = Makra
macro-play-title = Jak se má přehrát
macro-play-once = Jednou, od místa, kde stojí kurzor
macro-play-each = Jednou pro každý označený soubor
macro-sure-title = Přehrát toto makro
macro-sure-body = Makro na { $key } kopíruje, přesouvá, maže nebo přepisuje. Přehrát ho?
macro-too-deep = makro nemůže přehrávat samo sebe donekonečna
plugin-list-title = Příkazy modulů
plugin-none-title = Nejsou nainstalovány žádné moduly
plugin-none-open = Otevřít adresář modulů ({ $dir })
plugin-none-help = Jak moduly fungují a kde je najít
plugin-help-heading = Moduly
plugin-why-off = vypnut v Nastavení → Nastavení modulů
plugin-why-panel = jen v panelech
plugin-why-viewer = jen v prohlížeči
plugin-why-editor = jen v editoru
plugin-why-local = potřebuje adresář na discích tohoto počítače
plugin-why-files = potřebuje soubor pod kurzorem nebo označený
plugin-why-masks = jen pro { $masks }
plugin-why-bad-mask = jeho maska je chybná: { $error }
plugin-why-here = potřebuje { $names } v tomto adresáři
plugin-why-inside = potřebuje { $names } zde nebo v adresáři výše
plugin-cannot = { $label }: { $why }
plugin-running = { $label }…
plugin-on-selected = pro označené soubory: { $count }
plugin-failed = { $id }: { $why }
plugin-unknown-command = modul požádal o `{ $id }`, což není příkaz
hex-copied = { $size } zkopírováno z { $from }
hex-moved = { $size } přesunuto z { $from }
menu-calculator = Kalkulačka
menu-terminal = Terminál
calc-title = Kalkulačka
calc-decimal = desítkově
calc-hex = šestnáctkově
calc-octal = osmičkově
calc-binary = dvojkově
calc-characters = znaky

# The archive commands (Shift+F3) and the two parts of the order that are not the key.
arc-commands-title = Příkazy archivu
arc-password = Heslo pro { $name }
arc-op-test = Otestovat { $name }
arc-op-extract-here = Rozbalit { $name } sem
arc-op-extract-to = Rozbalit { $name } do…
arc-op-delete = Smazat { $name }
arc-op-add = Přidat soubory do archivu…
job-test = Testování archivu
msg-dirs-first-on = nejdříve adresáře
msg-dirs-first-off = adresáře mezi soubory
msg-selected-first-on = nejdříve označené soubory
msg-selected-first-off = označené soubory v pořadí

# The change-drive dialog (Alt+F1, Alt+F2): the type column, and its footer.
drive-fixed = Disk
drive-removable = Vyměnitelný
drive-optical = Optický
drive-network = Síťový
drive-memory = Paměť
drive-home = Domovský
drive-connection = Připojení
drive-objects = Úložiště objektů
drive-netbox = NetBox: uložené relace
drive-processes = Seznam procesů
drive-temp = Dočasný panel
temp-added = přidáno do dočasného panelu: { $count }
proc-none = seznam běžících procesů tu nelze přečíst
drive-programs = Nainstalované programy
drive-services = Služby
svc-none = tento počítač nemá správce služeb, kterého bychom se mohli zeptat
temp-not-a-file = řádky tady nejsou soubory: není co mazat
svc-card = Služba
svc-card-id = Název
svc-card-name = Zobrazuje se jako
svc-card-state = Stav
svc-card-description = Popis
svc-card-pid = Proces
svc-state-running = běží
svc-state-stopped = zastavena
svc-state-failed = selhala
svc-state-other = jiný
key-service-start = Start
key-service-restart = Restart
key-service-stop = Stop
svc-ask-stop = Zastavit { $name }?
svc-ask-restart = Restartovat { $name }?
btn-service-stop = Zastavit
btn-service-restart = Restartovat
svc-working = { $name }: dotaz na systém…
svc-started = { $name } spuštěna
svc-stopped = { $name } zastavena
svc-restarted = { $name } restartována
svc-needs-root = { $name }: toto vyžaduje root — program o něj za vás žádat nebude
svc-failed = { $name }: { $error }
prog-none = nic tu neříká, co je nainstalováno
prog-card = Nainstalovaný program
prog-no-uninstall = nic neříká, jak { $name } odebrat
prog-uninstall-ready = příkaz, který odebere { $name }, je na příkazovém řádku, nespuštěný
prog-card-name = Název
prog-card-version = Verze
prog-card-publisher = Vydavatel
prog-card-size = Velikost na disku
prog-card-installed = Nainstalováno
prog-card-location = Adresář
prog-card-uninstall = Odebírá se příkazem
drive-sockets = Seznam socketů
sock-none = nic tu nenaslouchá, nebo tabulku nelze přečíst
sock-no-process = nic neříká, který proces tento socket drží
proc-card = Proces
proc-gone = { $name } už neběží
proc-card-name = Název
proc-card-pid = ID procesu
proc-card-parent = Spuštěn procesem
proc-card-user = Uživatel
proc-card-state = Stav
proc-card-threads = Vlákna
proc-card-memory = Paměť
proc-card-mapped = Mapováno
proc-card-bits = Bitovost
proc-card-started = Spuštěn
proc-card-exe = Spustitelný soubor
proc-card-cwd = Adresář
proc-card-command = Příkazový řádek
proc-card-group = Skupina
proc-card-sid = SID
proc-card-priority = Priorita
proc-card-session = Relace
proc-card-uptime = Běží už
proc-card-cpu-user = Čas procesoru, vlastní
proc-card-cpu-kernel = Čas procesoru, v jádře
proc-card-handles = Otevřené handly
proc-card-faults = Výpadky stránek
proc-card-private = Soukromá paměť
proc-card-swap = Odloženo
proc-card-io-read = Přečteno
proc-card-io-write = Zapsáno
proc-card-memory-peak = Špička pracovní sady
proc-card-private-resident = Pracovní sada, soukromá
proc-card-swap-peak = Špička stránkovacího souboru
proc-card-virtual = Virtuální bajty
proc-card-virtual-peak = Špička virtuálních bajtů
proc-card-paged-pool = Stránkovaný fond
proc-card-nonpaged-pool = Nestránkovaný fond
proc-card-io-other = Ostatní V/V
proc-card-io-read-ops = Operace čtení
proc-card-io-write-ops = Operace zápisu
proc-card-io-other-ops = Ostatní operace
proc-kill-title = Ukončit proces
proc-kill-ask = Ukončit { $name } [{ $pid }]? Vše, co neuložil, bude ztraceno.
proc-killing = Ukončování { $name }…
proc-killed = { $name } ukončen
proc-kill-failed = { $name } se nepodařilo ukončit: { $why }
proc-card-env = Prostředí
proc-card-modules = Načtené moduly
proc-card-version = Verze
proc-card-description = Popis
drives-hint = ↑↓ výběr · 0-9 panely · Ctrl+1..9 sloupce · F9 všechny · Ctrl+R obnovit
drives-title = Změnit jednotku: co zobrazovat
drives-col-type = typ
drives-col-device = zařízení
drives-col-label = jmenovku
drives-col-fs = souborový systém
drives-col-size = velikosti
drives-col-removable = vyměnitelné jednotky
drives-col-panels = panely, které nejsou jednotkami
drives-col-optical = optické jednotky
drives-col-network = síťové sdílené složky
drives-shown = zobrazuje se: { $what }
drives-hidden = skrývá se: { $what }
drives-reread = jednotky byly načteny znovu
bookmarks-hint = Enter přejít · Ins uložit · Del odebrat · F4 pojmenovat · Esc zavřít

# The search dialog (F7), which asks the whole question in one place.
search-title = Hledat
search-for = Hledat
search-mode = Jako
search-mode-text = text
search-mode-hex = hex
search-mode-regex = regulární výraz
search-case = Rozlišovat velikost písmen
search-words = Jen celá slova
btn-find-next = Najít další
btn-find-prev = Najít předchozí
# The help: the chapters' own names, and the note on a chapter nobody has translated yet
help-chapter-about = Co to je
help-chapter-panels = Panely
help-chapter-files = Soubory: kopírování, přesun, hledání, archivy
help-chapter-viewer = Prohlížeč
help-chapter-editor = Editor, šestnáctkový panel a assembler
help-chapter-network = Servery
help-chapter-config = Konfigurace, klávesy a barvy
help-chapter-keys = Klávesy, jak jsou právě přiřazeny
help-chapter-commands = Všechny příkazy a jejich klávesy
help-chapter-options = Všechna nastavení a jejich hodnoty
help-here-panels = Panely
help-here-viewer = Prohlížeč
help-here-editor = Editor
help-here-hex = Šestnáctkový editor
help-here-dialog = Tento dialog
help-in-english = Tato kapitola ještě nebyla přeložena do vašeho jazyka, proto následuje anglicky. Klávesy a nastavení, které jmenuje, jsou ve všech jazycích stejné.
help-about-byline = FinalCommander { $version } — autor: Matteo Baccan
key-ok = OK
key-cancel = Storno
key-dialog-move = Posun
key-tree = Strom
key-config = Nastav
browse-local-only = soubor se vybírá z tohoto počítače; cesta na serveru se píše ručně a Tab ji doplní
key-browse = Procház
table-prompt-filter = Řádky obsahující
table-sorted = seřazeno podle { $column }
table-sorted-back = seřazeno podle { $column }, v opačném pořadí
table-unsorted = zpět v pořadí, v jakém jsou v souboru
table-filtered = zbylo řádků: { $rows }
table-unfiltered = znovu všechny řádky
table-cut-sorted = z prvních { $rows } řádků, víc načteno nebylo
table-too-many = příliš mnoho záznamů na to, aby se tu daly řadit nebo filtrovat
table-copied =
    { $rows ->
        [one] jeden řádek ve schránce
        [few] { $rows } řádky ve schránce
       *[other] { $rows } řádků ve schránce
    }
table-headed = první řádek je záhlaví
table-not-headed = první řádek jsou data: sloupce se jmenují podle čísla
table-named-fields = tento formát si svá pole pojmenovává sám: žádné záhlaví není třeba hádat

# Splitting and combining (SPEC 03 §9)
split-title = Rozdělit soubor
split-target = Do
split-size = Každá část
split-size-hint = 1440K, 700M, 4G — poslední část je to, co zbude
split-count = Nebo na
split-count-hint = částí — číslo zde má přednost před velikostí
split-bad-count = { $count } není počet částí: dvě nebo více
split-local-file = Dělení a spojování pracují se souborem na tomto počítači
split-bad-size = „{ $size }“ není velikost: zapište ji jako 700M nebo 4G
combine-not-a-piece = To není část: části rozděleného souboru končí na .001, .002, … nebo mají u sebe .crc
pieces-working = Dělení nebo spojování: { $done } z { $total }
pieces-cancelled = Zastaveno a to, co bylo zapsáno, je odstraněno
split-done = zapsáno částí: { $parts } a vedle nich { $list }
combine-done-checked = { $name } spojen z částí: { $parts }, { $bytes }, a odpovídá svému .crc
combine-done-unchecked = { $name } spojen z částí: { $parts }, { $bytes } — nebyl žádný .crc, podle kterého by šel zkontrolovat
pieces-failed = Neprovedeno: { $error }

# The branch view (SPEC 02 §3d)
branch-reading = Načítání všech souborů pod { $path }…
branch-title = Větev: { $path }
branch-done = souborů: { $count }, ze všech adresářů pod tímto · Ctrl+Alt+B se vrací
branch-truncated = souborů: { $count }, a tam se procházení zastavilo: větev je větší, než může seznam rozumně ukázat
branch-not-here = Zobrazení větve se týká adresáře a tento panel je seznam

## The release check (SPEC 11 §5).
menu-check-update = Zkon&trolovat aktualizace
opt-update-check = Kontrolovat nové vydání jednou denně
btn-open-page = Otevřít stránku
update-checking = Dotaz na GitHub na nejnovější vydání…
update-title = Aktualizace
update-available = Vyšel FinalCommander { $version } (toto je { $current }).
update-available-status = Vyšel FinalCommander { $version }: Nastavení → Zkontrolovat aktualizace
update-current = Toto je nejnovější vydání, { $version }.
update-failed = Na nejnovější vydání se nepodařilo zeptat: { $why }

## Duplicate files (SPEC 06 §13).
menu-dupes = Hledat duplicitní soubory
dupes-title = Duplicitní soubory
dupes-recursive = Hledat v podadresářích
dupes-min-size = Nejmenší soubor
dupes-min-size-hint = Velikost, jak ji píší filtry: 1, 100K, 5M. Prázdné soubory se nikdy neporovnávají.
dupes-select = Označit všechny kopie kromě nejstarší
btn-search = Hledat
dupes-not-here = Hledání duplikátů začíná od adresáře, nebo od toho, co je označeno.
dupes-bad-size = Není velikost: { $size }
dupes-looking = Hledání duplicitních souborů…
dupes-progress = Duplikáty: průchod { $pass } ze 3, { $done }/{ $total }
dupes-none = Mezi soubory ({ $count }) nejsou žádné duplikáty.
dupes-panel = Duplikáty: skupin — { $groups }
dupes-done = skupin duplikátů: { $groups }, v kopiích { $size }.
dupes-truncated = Zastaveno předčasně: zatím skupin { $groups }, v kopiích { $size }.

## SQLite queries (SPEC 04 §4b).
sql-empty = Napište dotaz za sql:, například sql: SELECT * FROM t
sql-no-database = sql: potřebuje databázi SQLite: vstupte do ní, nebo na ni postavte kurzor.
sql-running = Provádění dotazu…
sql-failed = Dotaz selhal: { $why }

## Asking a language model (SPEC 06 §14).
ai-none = Není nastavena žádná služba AI: přidejte blok [[ai]] do fcmd.toml (name, url, model, api, token_env).
ai-title = Zeptat se AI
ai-no-thread = není odpověď, na kterou by šlo navázat: nejprve se zeptejte přes `ai:`, nebo postavte kurzor na odpověď v adresáři odpovědí
ai-nothing-to-explain = není tu co vysvětlovat: otevřete soubor v prohlížeči nebo editoru, nebo se zeptejte hned po příkazu, který selhal
ai-explain-question = Co je tohle, z { $name }? Vysvětli stručně.
ai-tokens = tokenů: { $in } dovnitř, { $out } ven
ai-none-panel = Zatím žádná služba AI — F7 ji přidá
ai-service-removed = { $name } odebrána
ai-probing = Přihlašování k { $name } na { $host }…
ai-probe-ok = { $host } odpověděl HTTP { $status } za { $ms } ms, modelů: { $models } — napište otázku a stiskněte Enter
ai-probe-no-model = (model { $model } mezi nimi není)
ai-row-context = kontext
ai-row-conversation = konverzace
ai-session-title = AI: { $name } · { $model } — souborů v kontextu: { $files }
ai-context-added = { $count ->
        [one] Jeden soubor
        [few] { $count } soubory
       *[other] { $count } souborů
    } v kontextu
ai-context-nothing = Není co přidat: do kontextu jdou jen soubory z tohoto disku
ai-command-ready = příkaz je na řádku — přečtěte si ho, než stisknete Enter
ai-commit-question = Napiš zprávu commitu pro změny připravené v indexu.
ai-staged-diff = diff změn v indexu
ai-send-diff = Diff změn v indexu se posílá na { $host }
ai-send-files = Souborů: { $count }, { $size } — budou odeslány na { $host } spolu s otázkou.
ai-asking = Dotaz na { $name }…
ai-busy = { $name } přemýšlí… { $secs } s · Esc zastaví
ai-conversation-new = Nová konverzace: zahájí ji příští otázka.
ai-live-empty = Napište otázku na příkazový řádek: půjde k { $name } a konverzace se tu bude průběžně zobrazovat.
ai-failed = AI neodpověděla: { $why }
ai-stopped = zastaveno: otázka pro { $name } byla zahozena a nic nebylo zapsáno
ai-context-title = Soubory v kontextu
ai-context-all = Všechny
ai-context-dropped = { $count ->
        [one] jeden soubor odebrán z kontextu
        [few] { $count } soubory odebrány z kontextu
       *[other] { $count } souborů odebráno z kontextu
    }
ai-context-hint = Enter nebo Del: odebrat z kontextu — soubor zůstává na disku · F3: co se posílá · Esc
ai-threads-title = Konverzace
ai-threads-hint = Enter: pokračovat v této konverzaci · Esc
ai-thread-resumed = pokračování { $name }
ai-next-estimate = další ≈ { $tokens }
ai-next-over = další ≈ { $tokens }, víc, než model pojme
ai-send-estimate = Celkem asi { $tokens } tokenů, včetně dosavadní konverzace.
ai-send-estimate-of = Celkem asi { $tokens } tokenů, včetně dosavadní konverzace, z { $max }, které model pojme.
ai-send-over = Celkem asi { $tokens } tokenů: víc než { $max }, které model pojme, takže začátek může být oříznut. F8 odebere soubor.
ai-send-question = Otázka se posílá na { $host }.
btn-send = Odeslat

## Git (SPEC 06 §15).
menu-git = Git
git-title = Git
git-status = Co se změnilo
git-log = Historie
git-blame = Blame { $name }
git-branches = Větve
git-local-only = git pracuje s adresářem na tomto disku.
git-running = Dotaz na git…
git-failed = git: { $why }
git-clean = Nic se nezměnilo.
git-status-title = Změněno: v indexu { $staged }, mimo index { $unstaged }
git-side-conflict = konflikt
git-side-staged = v indexu
git-side-changed = mimo index
git-side-untracked = nesledováno
git-kind-modified = změněn
git-kind-added = nový
git-kind-deleted = smazán
git-kind-renamed = přejmenován
git-kind-copied = zkopírován
git-kind-type = změněn typ
git-kind-conflict = k vyřešení
git-as-tree = Zobrazit změny jako strom
git-as-list = Zobrazit změny jako seznam
git-version-head = poslední commit
git-version-index = v indexu
git-version-disk = na disku
git-discard-title = Zahodit změny
git-discard-body = Zahodit změny { $count ->
        [one] jednoho souboru
       *[other] { $count } souborů
    }?
git-discard-worktree = zpět k indexu:
git-discard-both = zpět k poslednímu commitu:
git-discard-added = z indexu, zůstává na disku:
git-discard-trash = do koše:
git-discard-warning = Co se vrací k indexu nebo k poslednímu commitu, nelze vzít zpět.
git-hint = Enter: vybrat · Esc: zavřít

## The editor's character list and word completion.
charmap-title = Znaky
charmap-hint = Psaním zúžit (znak nebo jeho kód, U+00E9) · Enter: vložit · Esc: zpět
editor-no-completion = Žádné jiné slovo v souboru tak nezačíná.

## A stored session's address, copied (NetBox's Generate URL).
net-url-copied = Zkopírováno { $url }

## The system log (SPEC 06 §16).
menu-syslog = Systémový protokol
syslog-reading = Načítání systémového protokolu…

## Deleting lines by pattern (FAR #10).
editor-prompt-delete-matching = Odstranit řádky odpovídající (! na začátku: ponechat jen ty)
editor-lines-deleted = Odstraněno řádků: { $count }

## The copy dialog's size policy (FAR #493).
copy-conflict-size = přepsat, liší-li se velikost

# Copy dialog: what an error does (FAR's issue #597)
copy-on-error = Pokud soubor nejde zkopírovat
copy-error-ask = Zeptat se
copy-error-skip = Přeskočit a pokračovat
copy-error-stop = Zastavit kopírování

# Attributes dialog: the times copied from one file to another (FAR's issue #241)
attr-times-copied = Časy zkopírovány: Ctrl+Shift+V je vloží do dialogu jiného souboru
attr-times-none = Zatím nic zkopírováno: nejprve Ctrl+Shift+C v dialogu souboru
attr-times-pasted = Časy vloženy: OK je nastaví

# Editor: replacements kept by name and recent ones (FAR's issue #282)
replace-tpl-title = Nahrazení
replace-tpl-hint = Enter spustí · Ins uloží nedávné pod názvem · Del zapomene uložené · Esc
replace-tpl-name = Uložit toto nahrazení jako
replace-tpl-kept = Uloženo jako {$name}
replace-tpl-empty = Zatím žádné nahrazení: nejprve Ctrl+F7

# Panel settings: a view and a sort per folder (FAR's issue #389)
opt-remember-view = Každý adresář si drží vlastní zobrazení a řazení

# Session editor: a cap on a copy's speed to or from the site
site-max-kbps = Limit rychlosti, KB/s (prázdné: žádný)

# Session editor: IMAP, read only
site-protocol-imap = IMAP (pošta, jen pro čtení)
site-protocol-imaps = IMAPS (pošta přes TLS, jen pro čtení)

# The AI services as a list, like the sessions
ai-services-title = Služby AI
ai-services-hint = Enter: té se ptá ai: · Ins nová · F4 změnit · Del odebrat · Esc
ai-service-title = Služba AI
ai-field-name = Název
ai-field-api = Protokol
ai-field-url = Adresa
ai-field-model = Model
ai-field-token-env = Klíč v proměnné
ai-field-token-note = Prázdné: na klíč se program zeptá jednou za relaci a nikdy ho nezapíše.
ai-field-system = Co se jí řekne nejdřív
ai-service-incomplete = Služba potřebuje název a adresu
ai-service-default = ai: se teď ptá {$name}
ai-key-prompt = Klíč pro {$name} (drží se do ukončení)
ai-api-openai = OpenAI a kompatibilní
ai-api-anthropic = Anthropic
ai-api-ollama = Ollama (na tomto počítači)
menu-ai = Služby AI

# The secrets file
vault-password = Heslo k souboru tajemství fcmd
vault-new = Uložit tento klíč do šifrovaného souboru tajemství fcmd: zvolte jeho heslo (prázdné: jen pro tuto relaci)
vault-refused = Soubor tajemství se neotevřel: {$why}

# The programs from elsewhere
drive-tools = Externí programy
drive-ai = AI
drive-git = Git
tools-found = nalezen
tools-missing = není nainstalován
tools-summary = nalezeno externích programů: { $found } z { $total }; F3 řekne, k čemu který je, F4 nainstaluje chybějící
tools-card = Externí program
tools-card-program = Program
tools-card-where = Kde
tools-card-version = Verze
tools-card-for = K čemu slouží
tools-card-install = Jak nainstalovat
tools-card-why = Proč ho fcmd spouští, místo aby to dělal sám: F1, „Programy odjinud“.
tools-install-any = nainstalujte balíček, který poskytuje { $name }, a ujistěte se, že je v PATH
tools-already = { $name } už tu je
tools-for-archives = archivy: { $what }
tools-for-decompile = Shift+F6 v prohlížeči na { $what }
tools-for-git = panel Git (Ctrl+Alt+G) a větev ve výzvě
tools-for-video = snímky videa v prohlížeči a rychlém náhledu (F3, Ctrl+Q)
tools-for-open = otevření odkazu v prohlížeči webu
tools-for-syslog = Příkazy → Systémový protokol
tools-for-services = panel služeb
tools-card-site = Web
tools-no-package = O žádném správci balíčků na tomto počítači není známo, že by měl { $name }. Otevřít stránku ke stažení?
tools-install-ask = Nainstalovat { $name } spuštěním v terminálu:
tools-install-how = Právě v terminálu se sudo zeptá na vaše heslo. Ctrl+Alt+E se vrací k panelům a Ctrl+R v tomto panelu zkontroluje znovu.
tools-install-else = Pokud to selže: { $url }
tools-install-failed = fcmd: { $name } nebyl nainstalován. Stáhněte ho z { $url }
tools-installing = instalace { $name } v terminálu: Ctrl+Alt+E se vrací k panelům, Ctrl+R tam zkontroluje znovu
btn-install = Instalovat

# Plugins configuration
plugins-config-title = Moduly
plugins-config-hint = Enter: zapnout nebo vypnout · F3 co to je a co přináší · Esc
plugins-broken = nenačetl se — F3 řekne proč
plugins-cannot-toggle = modul, který se nenačetl, nelze zapnout: F3 řekne, co s ním je
plugins-on = { $id } je zapnut: jeho příkazy jsou teď v Ctrl+Shift+F11
plugins-on-restart = { $id } je zapnut: jeho příkazy jsou teď v Ctrl+Shift+F11, zbytek přinese od příštího spuštění
plugins-off = { $id } je vypnut: jeho příkazy z Ctrl+Shift+F11 zmizely
plugins-off-restart = { $id } je vypnut: jeho příkazy z Ctrl+Shift+F11 zmizely, zbytek zmizí při příštím spuštění
plugins-state-on = zap
plugins-state-off = vyp
plugins-card-id = Id
plugins-card-name = Název
plugins-card-version = Verze
plugins-card-author = Autor
plugins-card-about = Popis
plugins-card-dir = Adresář
plugins-card-state = Stav
plugins-card-brings = Přináší
plugins-card-commands = Příkazy (Ctrl+Shift+F11):
plugins-card-notes = Hlášení při načítání:
plugins-none = žádné moduly nenalezeny · bydlí v { $dir }, každý ve vlastním adresáři s plugin.toml

# The git panel
git-file-log = Historie { $name }
git-commit = Commitnout, co je v indexu…
git-pull = Pull
git-push = Push
git-pushing = Probíhá push…
git-pulling = Probíhá pull…
git-commit-prompt = Zpráva commitu
git-commit-empty = Commit potřebuje zprávu.
git-already-on = { $branch } je větev, na které jste.
git-switch-ask = Přepnout na { $branch }? Soubory, ve kterých se obě větve liší, se na disku změní.
git-done = git: { $said }
git-branches-title = Větve: { $count }
git-log-title = Historie: { $count }
git-log-of-title = Historie { $of }: { $count }
btn-switch = Přepnout
key-git = Git
key-git-diff = Diff
key-git-stage = Index
key-git-unstage = ZIndexu
key-git-discard = Zahodit
key-git-commit = Commit
key-git-show = Ukázat
key-git-log = Histor
key-git-changes = Změny
key-git-pull = Pull
key-git-push = Push
key-git-branches = Větve

undo-title = Zpět
btn-undo = Zpět
menu-undo = Vrátit poslední operaci
menu-strip-meta = Odstranit metadata obrázků…
undo-nothing = není co vracet: od spuštění programu neproběhlo žádné kopírování, přesun, smazání do koše ani nový adresář
undo-copy =
    { $count ->
        [one] Poslat do koše soubor, který vytvořilo kopírování:
        [few] Poslat do koše { $count } soubory, které vytvořilo kopírování:
       *[other] Poslat do koše { $count } souborů, které vytvořilo kopírování:
    }
undo-move =
    { $count ->
        [one] Přesunout soubor zpět tam, kde byl:
        [few] Přesunout { $count } soubory zpět tam, kde byly:
       *[other] Přesunout { $count } souborů zpět tam, kde byly:
    }
undo-trash =
    { $count ->
        [one] Vrátit soubor z koše:
        [few] Vrátit { $count } soubory z koše:
       *[other] Vrátit { $count } souborů z koše:
    }
undo-mkdir = Odstranit nový adresář, pokud je stále prázdný:
strip-title = { $count ->
        [one] Metadata jednoho obrázku
       *[other] Metadata { $count } obrázků
    }
strip-all = Vše — text, EXIF, XMP, IPTC, komentáře, C2PA
strip-ai = Jen to, co zapsal obrazový model — prompt, workflow, nastavení
strip-camera = Vše kromě EXIF fotoaparátu a copyrightu — bez GPS
strip-hint = Enter: odstranit; obrázek se znovu nekóduje, Ctrl+Alt+Z to vrátí · Esc
strip-nothing = žádný PNG, WebP ani JPEG na tomto disku není označen ani pod kurzorem
strip-done = { $count ->
        [one] jeden obrázek
        [few] { $count } obrázky
       *[other] { $count } obrázků
    } bez metadat (odstraněno bloků: { $blocks }), bez čehokoli k odstranění: { $clean } · Ctrl+Alt+Z je vrátí
undo-restore = { $count ->
        [one] Vrátit obrázek s jeho metadaty:
        [few] Vrátit { $count } obrázky s jejich metadaty:
       *[other] Vrátit { $count } obrázků s jejich metadaty:
    }
undo-done =
    { $count ->
        [one] jeden název zpět tam, kde byl
        [few] { $count } názvy zpět tam, kde byly
       *[other] { $count } názvů zpět tam, kde byly
    }
slow-drives = seznam jednotek trval { $secs } s
slow-drives-late = seznam jednotek čekal { $secs } s: { $drives } neodpověděly včas a jsou zobrazeny bez podrobností — síťová jednotka, jejíž server tu není?
slow-listing = načtení { $what } trvalo { $secs } s
# editor search and undo (0.41)
btn-find-all = Všechny shody
btn-replace-all = Nahradit vše
btn-replace-one = Nahradit další
editor-replace-title = Nahradit
editor-replace-style = Zachovat styl (foo → bar, Foo → Bar, FOO → BAR)
editor-replace-groups = V regulárním výrazu jsou $1 a { "${name}" } to, co zachytila skupina, $$ je dolar
editor-matches-title = Řádky s `{ $query }`: { $count }
editor-matches-hint = Psaním zúžit · Enter: jít na řádek · Esc: zpět


# sorting and extended attributes (0.41)
opt-sort-case = Velká a malá písmena při řazení
opt-sort-case-auto = jako tento systém
opt-sort-case-yes = rozlišovat
opt-sort-case-no = jedna abeceda
copy-keep-xattrs = Zachovat rozšířené atributy

# editor conversions, disk diff and guides (0.41)
editor-convert-title = Zapsat soubor jako
editor-convert-hint = Psaním zúžit · Enter: převést (F2 to zapíše, Ctrl+Z vrátí) · Esc: zpět
editor-convert-eol = Konec řádku: { $eol }
editor-convert-bom-on = Značka pořadí bajtů (BOM): zapisuje se
editor-convert-bom-off = Značka pořadí bajtů (BOM): nezapisuje se
editor-convert-page = Kódová stránka: { $page }
editor-converted-eol = { $name }: při příštím uložení budou řádky končit na { $what }
editor-converted-bom-on = { $name }: příští uložení zapíše značku pořadí bajtů
editor-converted-bom-off = { $name }: příští uložení značku pořadí bajtů nezapíše
editor-converted-page = { $name }: příští uložení zapíše { $what }
editor-convert-same = Soubor už je takto zapsán
editor-convert-no-bom = { $page } nemá značku pořadí bajtů: mají ji jen UTF-8 a UTF-16
editor-convert-lossy = { $page } nepojme všechny znaky { $name }: nic se nezměnilo
btn-reload = Načíst znovu
btn-what-changed = Co se změnilo
editor-diff-buffer = v editoru
editor-diff-hint = Toto porovnání jen ukazuje, co se změnilo — Esc se vrací k otázce


# job journal and saved syncs (0.41)
journal-left = Kopírování nedokončených minulou relací: { $count } · Alt+Shift+F6 je vypíše
journal-title = Nedokončené úlohy
journal-hint = Enter pokračovat · Del zahodit · Esc zavřít
journal-none = Žádná úloha nezůstala nedokončená
journal-row = { $kind }: { $what } → { $target } · hotových souborů: { $done }
journal-row-unreadable = nelze přečíst: { $file } ({ $error })
journal-unreadable = { $file }: není to žurnál úlohy, který tato verze umí číst ({ $error }). Alt+Shift+F6 ho vypíše a Del ho tam odstraní
journal-resumed = Pokračování { $kind }: hotových souborů už bylo: { $done }
journal-discarded = Nedokončená úloha byla zahozena i s tím, co zapsala napůl
journal-cannot-resume = Tento žurnál nelze přečíst; Del ho odstraní
menu-jobs-left = Nedokončené úlohy
sync-report = Zapsat zprávu o průběhu do (soubor)
sync-name = Uložit tuto dvojici jako (název)
sync-saved-button = Uložené…
sync-saved-title = Uložené synchronizace
sync-saved-hint = Enter spustit · F4 změnit · Del zapomenout · Esc zavřít
sync-saved-none = Zatím není uložena žádná synchronizace: dejte jí název v dialogu (Ctrl+Shift+Y)
sync-saved-removed = { $name } už není uložena
sync-report-local = Zpráva musí být soubor na tomto počítači
sync-report-written = Zpráva o synchronizaci je v { $path }
sync-report-not-written = Zprávu se nepodařilo zapsat do { $path }: { $error }
sync-report-title = Zpráva o synchronizaci
sync-report-name = Název: { $name }
sync-report-left = Levý: { $path }
sync-report-right = Pravý: { $path }
sync-report-to-right = Zkopírováno zleva → doprava: { $count }
sync-report-to-left = Zkopírováno zprava → doleva: { $count }
sync-report-cut-right = Odstraněno vpravo: { $count }
sync-report-cut-left = Odstraněno vlevo: { $count }
sync-report-skipped = Ponecháno, ačkoli se liší: { $count }
sync-report-failed = Selhalo: { $count }
sync-report-stopped = Zastaveno v půli, takže mohly i nemusely být provedeny: { $count }
sync-report-not-done = Neprovedeno, protože dřívější krok selhal nebo byl zastaven: { $count }
menu-sync-saved = Uložené synchronizace

# hex editor search
hex-search-wrapped = `{ $query }`: nalezeno po novém začátku z druhého konce souboru
hex-replace-title = Nahradit
hex-replace-with = Nahradit čím
btn-replace = Nahradit
hex-replace-ask = Nahradit { $was } na pozici { $at } hodnotou { $with }?
hex-replace-length = náhrada má { $with } bajtů a shoda { $was }: to by posunulo vše za ní — nejprve stiskněte Ins pro režim vkládání

# pixels and video (0.41)
opt-viewer-pixels = Obrázky
opt-viewer-pixels-auto = skutečné pixely tam, kde je terminál kreslí
opt-viewer-pixels-kitty = grafický protokol kitty
opt-viewer-pixels-iterm = vložené obrázky iTerm2
opt-viewer-pixels-sixel = sixel
opt-viewer-pixels-off = vždy půlbloky
video-needs-program = toto je video ({ $what }): jeho snímky se kreslí přes `{ $program }`, který na tomto počítači není nainstalován — Shift+F11 i tak čte kontejner
video-no-frame = z tohoto videa není žádný snímek: { $why }
video-playing = přehrávání rychlostí { $fps } snímků za sekundu, bez zvuku — Space pozastaví
video-paused = pozastaveno na { $at } — Space přehrává dál, PgUp a PgDn krokují po desetině
video-ended = video skončilo — Space ho přehraje znovu


# finding things
menu-saved-searches = Uložená hledání
drive-searches = Uložená hledání
find-keys = F2 uloží toto hledání pod názvem · F9 uložená
find-save-name = Uložit toto hledání jako
find-saved = hledání uloženo jako { $name }
find-saved-none = zatím nebylo uloženo žádné hledání: F2 v dialogu hledání ho uloží
find-saved-title = Uložená hledání — Enter otevře, F4 upraví, Del zapomene
find-saved-running = hledání { $name }…
find-saved-gone = hledání { $name } zapomenuto
find-loaded = hledání { $name } načteno
find-fuzzy = Přibližný název: písmena výše, v pořadí, nejlepší shoda první
find-meta = Metadata (artist=bowie; camera=)
find-index = Zeptat se indexu systému místo procházení (může minout nové soubory)
tools-for-index = index, kterého se Alt+F7 může zeptat místo procházení
menu-frecent = Časté adresáře
frecent-none = zatím nebyl navštíven žádný adresář
frecent-gone = { $path } odebrán z častých adresářů
frecent-hint = pište písmena cesty · Enter jít tam · Del zapomenout · Esc zavřít

# ai third round
ai-redact-off = Maskování je vypnuto: vše se posílá tak, jak je
ai-redact-none = Nenalezeno žádné tajemství k zamaskování
ai-redact-count = Tajemství zamaskovaných před odesláním: { $count } ({ $kinds })
ai-redacted = zamaskovaných tajemství: { $count }
ai-preview-title = Co se posílá k { $name } ({ $host })
ai-preview-instruction = Instrukce
ai-preview-earlier = Dosavadní konverzace
ai-preview-question = Otázka a její soubory
ai-preview-key = F3 ukáže, co se posílá
ai-preview-reading = Načítání toho, co by se odeslalo…
key-ai-redact = Maskov
ai-redact-now-on = Maskování je zapnuto: klíče, tokeny, soukromé klíče a hesla se zamaskují dřív, než se cokoli odešle
ai-redact-now-off = Maskování je vypnuto: co se posílá, jde tak, jak je
ai-panel-redact-on = tajemství zamaskována
ai-panel-redact-off = tajemství nezamaskována
key-ai-local = Místní
ai-row-refused = není tento počítač: odmítnuto
ai-panel-local-only = jen místně
ai-local-now-on = Režim „jen místně“ je zapnut: služba, která není na tomto počítači, se odmítne a nic se jí nepošle
ai-local-now-off = Režim „jen místně“ je vypnut: lze se ptát každé služby v seznamu
ai-local-refused = Režim „jen místně“ je zapnut: { $name } odpovídá na { $host }, což není tento počítač — nic nebylo odesláno (F6 v panelu AI ho vypne)
ai-rename-prompt = Čím se mají názvy ({ $count }) stát? (slovy)
ai-send-names = Instrukce a { $count } z { $total } názvů — jen názvy, nic ze souborů — se posílají na { $host }.
ai-rename-ready = vzor od modelu je v dialogu: přečtěte si náhled, opravte, co je špatně, a přejmenuje až OK
ai-rename-waiting = vzor přejmenování od modelu je připraven: Ctrl+Shift+F6 s ním otevře dialog
ai-rename-unreadable = model neodpověděl vzorem přejmenování: jeho odpověď je v adresáři odpovědí (ai:) a nic nebylo přejmenováno
ai-rewrite-prompt = Co se má stát s označeným blokem? (slovy)
ai-rewrite-no-block = nejprve označte blok (Shift+šipky): přepis nahradí přesně to, co je označeno, a sloupcový blok nemá tvar, do kterého by se dalo vrátit
ai-rewrite-read-only = tento soubor je tu jen pro čtení: přepis by neměl kam jít
ai-rewrite-before = blok
ai-rewrite-after = jak ho přepsal model
ai-rewrite-hint = Přepis od modelu proti bloku — Enter ho vloží na místo bloku, Esc nechá blok, jak je
diff-hint-rewrite = Enter nahradit blok · Esc nechat
ai-rewrite-applied = blok byl nahrazen přepisem od modelu — jedno Ctrl+Z ho vrátí
ai-rewrite-left = blok byl ponechán, jak je
ai-rewrite-stale = blok už není takový, jaký byl při žádosti o přepis: nic nebylo nahrazeno — verze modelu je v adresáři odpovědí (ai:)
ai-rewrite-same = model vrátil blok beze změny: není co nahrazovat
ai-answer-kept = odpověď je { $name } v adresáři odpovědí (ai:) — editor zůstal vpředu
ai-failure-question = Tento příkaz skončil s návratovým kódem { $code }: `{ $command }` — proč selhal a co by to opravilo? Stručně.
ai-send-failure = Příkaz, který selhal, a poslední řádky jeho výstupu ({ $lines }) se posílají na { $host }.

# archives and jobs
arc-test-whole = { $name }: otestováno položek: { $files }, { $bytes } — všechny v pořádku
arc-test-bad-title = { $name }: { $bad } z { $files } položek nevyšlo v pořádku

# extending it
plugin-column-failed = { $column }: { $error } — jeho sloupec zůstává prázdný u souborů, na které neodpověděl
plugins-card-columns = Sloupce (jak je jmenuje [[column_set]]):
usermenu-far-row = Co bylo načteno jinak než z FarMenu.ini ({ $count })
usermenu-far-notes = menu pochází z FarMenu.ini z FARu: věcí načtených jinak: { $count } — poslední řádek je vyjmenuje
usermenu-far-not-run = Nic nebylo spuštěno.
find-age = Změněno, před dny (7, >=30, 7-30)
find-bad-age = `{ $text }` není stáří ve dnech — { $error }
find-label-age = { $days } d
find-keys-ai = Ctrl+Alt+Q popíše hledání slovy: jazykový model vyplní tyto řádky
ai-find-prompt = Co hledáte? (slovy)
ai-send-words = Tato slova — a nic z panelů: žádný název, cesta ani soubor — se posílají na { $host }.
ai-find-ready = hledání od modelu je v dialogu: přečtěte si řádky, opravte, co je špatně, a hledat začne až OK
ai-find-waiting = hledání od modelu je připraveno: Alt+F7 s ním otevře dialog
ai-find-unreadable = model neodpověděl hledáním: jeho odpověď je v adresáři odpovědí (ai:) a nic se nehledalo
ai-find-left-out = { $said } — NENÍ v řádcích, protože to vyjádřit neumějí: { $what }
# treemap and compare
usage-map-title = Využití disku, celý strom: { $path }
usage-map-walking = Měření… souborů: { $files }, zatím { $size }
usage-map-where = { $path } · { $size } · souborů: { $files }
usage-map-unreadable = nepřečtených adresářů: { $count }
usage-map-rest = menších souborů: { $count }
usage-map-others = ostatních: { $count }
usage-map-empty = Nic v tomto adresáři nezabírá žádné místo
usage-map-chosen = { $name } · { $size } · { $share }%
usage-map-hint = ←↑↓→ · Enter dovnitř · BkSp nahoru · Tab seznam · G panel tam · Esc
usage-map-hint-list = ↑↓ · Enter dovnitř · BkSp nahoru · Tab mapa · G panel tam · Esc
usage-map-hint-walking = Esc zastaví
usage-map-top = Toto je adresář, který se měřil: nad ním tu nic není
usage-map-no-file = Tento řádek je mnoho malých souborů, ne jeden, na který by panel mohl přejít
bin-comparing = Porovnávání obou souborů bajt po bajtu… Esc zastaví
bin-bytes = bajtů: { $count }
bin-same = Oba soubory jsou stejné, bajt po bajtu: každý { $bytes } bajtů
bin-where = rozdíl { $at } z { $count } · první na pozici { $offset } · lišících se bajtů: { $differing }
bin-hint = Alt+↑↓ rozdíly · ↑↓ PgUp PgDn Home End řádky · Esc zavřít
diff-unified-on = Jeden sloupec: co odešlo, potom co přišlo
diff-unified-off = Dva sloupce, vedle sebe
diff-patch-title = Uložit rozdíly jako patch (.diff)
diff-patch-saved = { $name } zapsán: sjednocený diff, řádků: { $lines }
diff-patch-nothing = Oba soubory jsou stejné: není žádný patch k uložení
# wastebasket and external editor
drive-trash = Koš
drive-trash-closed = Koš (tento systém nedovolí programu ho vypsat)
menu-trash = Koš
menu-trash-empty = Vysypat koš
key-trash-restore = Obnovit
key-trash-empty = Vysypat
btn-empty = Vysypat
btn-rename = Přejmenovat
trash-closed = koš tohoto systému lze otevřít jen z jeho vlastního správce souborů
trash-empty = koš je prázdný
trash-failed = koš: { $why }
trash-restored =
    { $count ->
        [one] jedna položka zpět tam, kde byla
        [few] { $count } položky zpět tam, kde byly
       *[other] { $count } položek zpět tam, kde byly
    }
trash-purged =
    { $count ->
        [one] jedna položka smazána nadobro
        [few] { $count } položky smazány nadobro
       *[other] { $count } položek smazáno nadobro
    }
trash-card-name = Název
trash-card-from = Byl v
trash-card-deleted = Smazáno
trash-card-size = Velikost
trash-clash-title = Obnovení
trash-clash-body = { $path } už existuje.
trash-clash-note = „Přepsat“ pošle ten existující do koše; „Přejmenovat“ vrátí tento vedle něj.
trash-clash-more =
    { $count ->
        [one] Ještě jedna položka je ve stejné situaci.
        [few] Ještě { $count } položky jsou ve stejné situaci.
       *[other] Ještě { $count } položek je ve stejné situaci.
    }
trash-purge-title = Smazat nadobro
trash-purge-body =
    { $count ->
        [one] Smazat { $name } z koše?
        [few] Smazat { $count } položky z koše?
       *[other] Smazat { $count } položek z koše?
    }
trash-purge-note = Toto maže natrvalo: nic je nevrátí.
trash-empty-title = Vysypat koš
trash-empty-body = Smazat vše, co je v koši?
opt-editor-external = Externí editor
opt-editor-use-external = Používat externí editor pro F4 (Alt+F4 je pak ten vestavěný)
opt-viewer-external = Externí prohlížeč
opt-viewer-use-external = Používat externí prohlížeč pro F3 (Alt+F3 je pak ten vestavěný)


# editor features
opt-editor-line-numbers = Číslovat řádky
editor-line-numbers-on = Čísla řádků zapnuta
editor-line-numbers-off = Čísla řádků vypnuta
editor-prompt-pipe-block = Prohnat blok příkazem:
editor-prompt-pipe-file = Prohnat celý soubor příkazem:
editor-prompt-insert-output = Vložit to, co tento příkaz vypíše:
editor-prompt-insert-file = Vložit soubor:
editor-prompt-write-block = Zapsat blok do souboru:
editor-pipe-running = Běží { $command } …
editor-pipe-done = { $command }: nahrazeno tím, co vypsal, řádků: { $lines } (Ctrl+Z to vrátí)
editor-pipe-inserted = { $command }: vloženo řádků: { $lines }
editor-pipe-empty = { $command } nic nevypsal: nic nebylo vloženo
editor-pipe-failed = { $command }: { $error } — nic se nezměnilo
editor-pipe-changed = { $command }: text se během běhu příkazu změnil — nic nebylo nahrazeno
editor-pipe-gone = { $command }: soubor, pro který byl spuštěn, už není otevřen — nic se nezměnilo
editor-pipe-column = Sloupcový blok nelze prohnat příkazem: označte celé řádky, nebo nic pro celý soubor
editor-file-inserted = { $name } vložen, řádků: { $lines }
editor-insert-not-local = Z editoru lze načíst nebo zapsat jen soubor na tomto počítači
editor-insert-too-big = příliš velký na načtení do editoru
editor-block-written = Blok zapsán do { $name }: { $size }
editor-block-write-self = { $name } je právě upravovaný soubor: blok přes něj zapsán nebyl
editor-choice-hint = Psaním zúžit · Enter: vybrat · Esc: zpět
editor-syntax-title = Obarvit soubor jako
editor-syntax-auto = Co říká jeho název: { $language }
editor-syntax-auto-none = Co říká jeho název: nic známého
editor-syntax-none = Prostý text, bez barev
editor-syntax-set = { $name } se teď čte jako { $language }
editor-syntax-off = { $name } se teď čte jako prostý text
editor-case-heading = Změnit velikost písmen
editor-case-upper = VELKÁ PÍSMENA
editor-case-lower = malá písmena
editor-case-title = Jako V Nadpisu: velké písmeno na začátku každého slova
editor-case-toggle = pŘEPNOUT: velká se stanou malými a malá velkými
editor-no-word = Nic není označeno a kurzor nestojí na slově
editor-prompt-isearch = Hledat při psaní (↓ ↑ další a předchozí, Esc se vrací)
editor-word-only = Slovo pod kurzorem není napsáno nikde jinde
editor-not-on-word = Kurzor nestojí na slově
editor-prompt-reflow = Na kolik sloupců odstavec přeformátovat? (samotný Enter: { $width })
editor-reflowed = Odstavec přeformátován na { $width } sloupců (Ctrl+Z to vrátí)
editor-reflow-bad-width = { $width } není šířka: počet sloupců od 8 do 1000
editor-no-paragraph = Kurzor stojí na prázdném řádku: žádný odstavec tu není
editor-no-name-yet = Tento soubor ještě nemá název: nejprve ho uložte
editor-tools-title = Příkazy editoru
editor-tool-isearch = Hledat při psaní
editor-tool-word-next = Další místo, kde je napsáno slovo pod kurzorem
editor-tool-word-prev = Předchozí místo, kde je napsáno slovo pod kurzorem
editor-tool-pipe = Prohnat blok nebo celý soubor příkazem
editor-tool-write-block = Zapsat blok do souboru
editor-tool-case = Změnit velikost písmen v bloku
editor-tool-reflow = Přeformátovat odstavec na danou šířku
editor-tool-insert-file = Vložit soubor na pozici kurzoru
editor-tool-insert-output = Vložit to, co vypíše příkaz
editor-tool-insert-date = Vložit datum a čas
editor-tool-insert-name = Vložit název tohoto souboru
editor-tool-insert-path = Vložit celou cestu tohoto souboru
editor-tool-line-numbers = Čísla řádků zapnout nebo vypnout
editor-tool-syntax = Zvolit jazyk, podle kterého se soubor obarvuje

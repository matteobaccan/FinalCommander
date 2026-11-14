# Key bar labels: short, they must fit in a few columns.
key-help = Súgó
key-menu = FhMenü
key-view = Megnéz
key-edit = Szerk.
key-copy = Másol
key-print = Nyomt
key-move = AtnMoz
key-mkdir = ÚjMapp
key-delete = Töröl
key-pulldown = KonfMn
key-quit = Kilép
# The key bar while a modifier is held: FAR's own words, six columns wide.
key-add = Tömört
key-extract = Kibont
key-arccmd = TömPar
key-edit-new = ÚjFájl…
key-rename = Átnev
key-group = Csoprt
key-selup = KijFel
key-left = Bal
key-right = Jobb
key-sort-name = Név
key-sort-ext = Kiterj
key-sort-write = Módos
key-sort-size = Méret
key-sort-unsort = NincsR
key-sort-create = Keletk
key-sort-access = Hozzáf
key-sort = RendMd
key-mklink = ÚjLink
key-find = Keres
key-history = ParElő
key-folders = MapElő
key-plugins = Plugin

# Panel chrome
list-up = Fel
list-folder = Mappa
col-name = Név
col-ext = Kit
col-size = Méret
col-allocated = Lefoglalt
col-modified = Módosítva
col-created = Létrehozva
col-accessed = Hozzáférés
col-attr = Attrib
col-owner = Tulajdonos
col-group = Csoport
col-descr = Megjegyzés
col-links = Lnk
col-git = Git
descr-title = { $name } megjegyzése
descr-set = { $name } megjegyzése beállítva
descr-cleared = { $name } megjegyzése törölve
descr-not-here = megjegyzések csak ezen a gépen lévő mappa mellett tárolhatók
col-date-time = Dátum      Idő
panel-list-count = { $count } elem a listában
panel-empty = (üres)
panel-loading = olvasás…

# Prompts
prompt-select-mask = Fájlok kijelölése maszk szerint:
prompt-deselect-mask = Kijelölés levétele maszk szerint:
prompt-goto = Mappaváltás ide:
prompt-filter = Fájlmaszk vagy névrészlet:
prompt-search = Gyorskeresés:

# Dialogs
dialog-error = Hiba
dialog-help-hint = Esc vissza a tartalomhoz · F7 keres minden fejezetben · F1 bezár · ↑↓ PgUp PgDn görget
dialog-help-index = FinalCommander { $version } — Súgó
dialog-help-hint-index = Enter megnyit egy fejezetet · F7 keres minden fejezetben · Esc bezár
help-search-prompt = Keresés a súgóban
help-search-none = A súgó egyetlen sora sem tartalmazza: „{ $query }”
help-search-title = „{ $query }” a súgóban: { $count } sor
output-search-prompt = Keresés a kimenetben
output-search-empty = Még semmi sem íródott ki
output-search-found = { $line }. sor / { $lines }
output-search-wrapped = { $line }. sor / { $lines }, újra a legfrissebbtől
output-search-none = A kimenet egyetlen sora sem tartalmazza: „{ $query }”

# Status messages
msg-not-implemented = { $command } ebben a változatban még nincs megvalósítva
msg-selected = { $count } elem kijelölve
msg-deselected = { $count } elemről levéve a kijelölés
msg-mask-invalid = érvénytelen maszk: { $error }
msg-theme = téma: { $name }
msg-font-bigger = betű: nagyobbat kértünk a termináltól
msg-font-smaller = betű: kisebbet kértünk a termináltól
msg-font-reset = betű: az eredetit kértük a termináltól
msg-sort = rendezés: { $key }{ $reverse }
msg-sort-groups-on = rendezési csoportok bekapcsolva ({ $count })
msg-sort-groups-off = rendezési csoportok kikapcsolva ({ $count })
msg-sort-groups-none = nincs rendezési csoport megadva: lásd [[sort_group]] a konfigurációban
msg-view = nézet: { $mode }
msg-hidden-on = rejtett fájlok mutatva
msg-hidden-off = rejtett fájlok elrejtve
msg-no-history = nincs több előzmény
msg-tab-last = egy panelnek mindig van egy füle
msg-tab-end = a fül már a sor azon végén van
tabs-title = Fülek
msg-tab-moved = a fül átkerült a másik panelre
msg-tab-locked = ez a fül a mappájához van zárolva
msg-tab-locked-on = a fül a mappájához zárolva
msg-tab-locked-off = a fül zárolása feloldva
msg-bookmark-set = { $digit }. gyorsbillentyű: { $path }
msg-bookmark-unset = a(z) { $digit }. gyorsbillentyű nincs beállítva
msg-bookmark-added = { $path } megjegyezve
msg-bookmark-removed = { $path } eltávolítva
msg-bookmark-known = ez a mappa már szerepel a listában
msg-dirsize = { $path }: { $bytes }, { $files } fájl
msg-dirsize-many = { $count } mappa mérése…
usage-title = Lemezhasználat: { $path } · { $size } · Tab: a teljes fa térképként
usage-not-here = egy mappa lemezhasználata lemezen mérhető, nem archívumban vagy találati listában
msg-dirsize-no-folder = itt nincs mérhető mappa

# Jobs: prompts, confirmations, progress, conflicts
prompt-copy-to = { $what } másolása ide:
prompt-move-to = { $what } átnevezése-mozgatása ide:
prompt-mkdir = Mappa neve:
prompt-tab-name = A fül neve:
prompt-bookmark-name = A mappa neve:
what-selected = { $count } kijelölt elem
confirm-delete-title = Törlés
confirm-delete-body =
    { $count ->
        [one] Törölni akarja: { $name } ({ $bytes })?
       *[other] Törölni akar { $count } elemet ({ $bytes })?
    }
prompt-link = Link létrehozása ide: { $name }, ezen a néven:
link-created = { $name } létrehozva
confirm-wipe-title = Kisöprés
confirm-wipe-body =
    { $count ->
        [one] Felülírja és törli: { $name } ({ $bytes })?
       *[other] Felülír és töröl { $count } elemet ({ $bytes })?
    }
confirm-wipe-honest = A bájtok egyszer felülíródnak, majd a fájl törlődik. SSD-n, copy-on-write fájlrendszeren vagy pillanatképek mellett régebbi másolatok megmaradhatnak — a név pedig megmarad a mappában és a naplóban.
job-copy = Másolás
job-move = Mozgatás
job-delete = Törlés
job-wipe = Kisöprés
job-trash = Áthelyezés a Lomtárba
job-replace = Csere
job-pack = Tömörítés
job-rebuild = Az archívum írása

# ── the attributes dialog (SPEC 10 §3.8) ───────────────────────────────────────────────
attr-title = Fájl attribútumok
attr-count = { $count } kijelölt elem
attr-now = most { $text } ({ $octal })
attr-owner-read = A tulajdonos olvashatja
attr-owner-write = A tulajdonos írhatja
attr-owner-exec = A tulajdonos futtathatja
attr-group-read = A csoport olvashatja
attr-group-write = A csoport írhatja
attr-group-exec = A csoport futtathatja
attr-other-read = Bárki olvashatja
attr-other-write = Bárki írhatja
attr-other-exec = Bárki futtathatja
attr-setuid = Futtatás a tulajdonos nevében (setuid)
attr-setgid = Futtatás a csoport nevében (setgid)
attr-sticky = Csak a tulajdonos törölheti (sticky)
attr-readonly = Csak olvasható
attr-hidden = Rejtett
attr-system = Rendszer
attr-archive = Archív
attr-temporary = Átmeneti
attr-offline = Offline
attr-indexed = Az indexelő megnézheti
attr-compressed = tömörített
attr-encrypted = titkosított
attr-sparse = ritkított
attr-reparse = reparse point
attr-also = továbbá { $what }
attr-mtime = Módosítás
attr-atime = Utolsó olvasás
attr-btime = Létrehozás
attr-ctime = Bejegyzés változása
attr-btime-readonly = (a létrehozás ideje csak Windowson állítható)
attr-utc = Idők UTC-ben
attr-owner = Tulajdonos (felhasználó:csoport)
attr-owner-is = tulajdonos: { $who }
btn-attr-original = Eredeti
btn-attr-current = Aktuális
btn-attr-blank = Üres
attr-recursive = A mappák teljes tartalmára is
attr-bad-time = `{ $text }` nem dátum és idő (ÉÉÉÉ-HH-NN ÓÓ:PP:MM)
attr-bad-owner = itt nincs `{ $text }` nevű fiók vagy csoport
attr-in-archive = ez a módosítás újraírja az archívumot, a többi bejegyzés változatlan marad
job-attrs = Attribútumok beállítása

# ── the copy dialog (SPEC 03 §4) ───────────────────────────────────────────────────────
copy-title = Másolás
copy-title-move = Átnevezés-Mozgatás
copy-target = Ide
copy-conflict = Már létező fájloknál
copy-conflict-ask = kérdez
copy-conflict-overwrite = felülír
copy-conflict-newer = felülír, ha a forrás újabb
copy-conflict-skip = kihagy
copy-conflict-rename = szabad nevet ad a másolatnak
copy-conflict-append = hozzáfűzi a végéhez
copy-only-newer = Csak az ott lévőknél újabb fájlokat
copy-filter = Csak az illeszkedő neveket
copy-follow-links = A link célját másolja, ne a linket
copy-keep-times = Időbélyegek megőrzése
copy-keep-mode = Jogosultságok megőrzése
copy-verify = Minden fájl visszaolvasása és összehasonlítása
copy-several = { $count } cél, egymás után

# ── the terminal (SPEC 06 §4) ──────────────────────────────────────────────────────────
term-started = terminál, benne: { $shell }
term-hint = Ctrl+Alt+E: vissza a panelekhez
term-ended = véget ért — bármely billentyű bezárja
job-mkdir = Mappa létrehozása
job-planning = tervezés…
job-counters = { $files } / { $filestotal } fájl · { $bytes } / { $bytestotal }
job-rates = { $speed }/s · eltelt: { $elapsed } · maradt: { $eta }
job-rates-no-eta = { $speed }/s · eltelt: { $elapsed }
job-hint = Esc megszakít
job-hint-queue = Esc megszakít · B háttérbe
job-queued = { $kind } a sorban vár, előtte { $ahead } befejezetlen feladat · Alt+Shift+F5 listázza
job-background = { $kind } a háttérben folytatódik · Alt+Shift+F5 listázza a feladatokat
job-asks = { $kind } válaszra vár · Alt+Shift+F5
job-paused = { $kind } szüneteltetve
job-resumed = { $kind } folytatódik
job-retry-nothing = Nincs mit újrapróbálni
job-still-waiting = Ez a feladat a sorára vár
jobs-title = Feladatok ({ $active } befejezetlen)
jobs-hint = Enter mutat · Space szünet · Del leállít · F5 újra · Ctrl+↑↓ sorrend · Esc bezár
jobs-quit-title = Még futnak feladatok
jobs-quit-body = Ezek a feladatok leállnak, ha a program most kilép:
job-state-running = fut
job-state-paused = szünetel
job-state-asking = kérdez
job-state-queued = várakozik
job-state-held = visszatartva
job-state-done = kész
job-state-failed = hibák
job-state-cancelled = megszakítva
job-done = { $kind }: { $files } fájl, { $bytes }
job-done-renamed = { $kind }: { $files } fájl másolás nélkül áthelyezve
job-done-skipped = , { $skipped } kihagyva
job-done-verified = , { $verified } ellenőrizve
job-done-unverified = , { $unverified } itt nem volt visszaolvasható
job-done-errors = , { $errors } hiba
job-cancelled = { $kind } megszakítva
job-errors-title = Hibák
conflict-title = A fájl már létezik
conflict-dst = létező   { $path }   { $bytes }
conflict-src = új       { $path }   { $bytes }
conflict-newer = a forrás újabb
conflict-older = a cél újabb
btn-overwrite = Felülír
btn-overwrite-all = Mindet
btn-skip = Kihagy
btn-skip-all = Kihagy mind
btn-retry = Újra
btn-move = Mozgat
btn-delete = Töröl
btn-wipe = Kisöpör
btn-copy = Másol
btn-discard = Elvet
btn-kill = Kilő
btn-print = Nyomtat
btn-quit = Kilép
btn-play = Lejátszás
btn-run = Futtat
btn-accept = Elfogad
btn-cancel = Mégsem
msg-nothing-to-do = nincs kijelölve semmi
cmd-complete-nothing = nincs mivel kiegészíteni
complete-several = { $count } név kezdődik így
form-moving = a nyilak mozgatják a párbeszédablakot · Enter vagy Esc ott hagyja
form-moved = a párbeszédablak ott marad, ahová tette
msg-trash-refused = A Lomtár nem fogadta be — a Shift+Del véglegesen töröl.

# Checksums (SPEC 03 §9)
sum-title = Ellenőrző összegek
sum-algorithm = Hash
sum-one-file = Egy lista mindegyikhez
sum-name = Név
sum-working = { $name }: { $done } / { $total }
sum-verifying = ellenőrzés { $name } alapján
sum-stopped = leállítva
sum-written = { $name } megírva
sum-no-name = a listának név kell
sum-local-only = ellenőrző összeg csak ezen a gépen lévő fájlból számolható
print-title = Nyomtatás
print-body = Kinyomtatja: {$what}?
print-sending = küldés a nyomtatóra…
print-sent = elküldve a nyomtatóra
print-failed = a nyomtatás nem sikerült ({$line}): {$error}
print-local-only = csak ezen a gépen lévő fájl nyomtatható
print-no-command = nincs nyomtatási parancs: állítsa be a [print] command értékét, vagy telepítse az lp vagy lpr programot
sum-all-match = mind a(z) { $count } egyezik
sum-verify-title = Ellenőrző összegek
sum-verify-summary = { $ok } egyezik, { $bad } nem
sum-differs = nem egyezik
sum-missing = nincs meg
sum-unreadable = nem olvasható

# Synchronising two directories (SPEC 03 §10)
sync-title = Szinkronizálás
sync-between = { $left }  ↔  { $right }
sync-recursive = Az almappák teljes tartalma is
sync-both-ways = Mindkét irányba, az újabb nyer
sync-delete-extra = Felajánlja annak törlését, ami a másik oldalon nincs meg
sync-by-content = A bájtokat hasonlítja, nem a méretet és az időt
sync-filter = Csak az illeszkedő neveket
sync-walking = vizsgálat: { $at } — eddig { $pairs }
sync-planned = { $copies } másolandó ({ $bytes }), { $deletes } törlendő
sync-hint = Space következő válasz · ← → irány · = marad · F2 végrehajt · Esc mégsem
sync-hint-walking = Esc leállítja a bejárást
sync-nothing-to-do = minden sor szerint minden marad, ahogy van
sync-local-only = a szinkronizálás csak ezen a gépen tud olvasni és írni
sync-same-directory = a két panel ugyanabban a mappában áll

# What to run for a kind of file (SPEC 06 §5)
assoc-none = semmi sem mondja meg, mivel nyitható meg: { $name }
usermenu-empty = Még nincs felhasználói menü: írjon egyet ide: { $path }
usermenu-last-gone = a legutóbb választott sor már nincs ebben a menüben
usermenu-bad-condition = Érthetetlen feltétel — { $problem }
assoc-desktop = Átadás az asztali környezetnek
assoc-open-with = { $name } megnyitása ezzel
assoc-no-pager = sem társítás, sem $PAGER nem mondja meg, mivel olvasható
assoc-no-editor = sem társítás, sem $VISUAL nem mondja meg, mivel szerkeszthető

# Viewer (F3)
key-wrap = SorTör
key-hex = Hexa
key-text = Szöveg
key-dump = Dump
key-doc = Dok
key-search = Keres
key-code = Kód
key-follow = Követ
viewer-prompt-search = Keresés ({ $mode }):
viewer-prompt-goto = Ugrás eltolásra vagy NN% :
viewer-not-found = Nem találtam: `{ $query }`
viewer-follow-on = a fájl végének követése
viewer-follow-off = követés kikapcsolva
viewer-codepage = kódlap: { $name }
viewer-wrap-mode = { $mode }
viewer-no-undo = nincs korábbi pozíció
viewer-empty = (üres fájl)
viewer-not-markdown = nem Markdown fájl, vagy túl nagy a megjelenítéshez
viewer-not-table = nem táblázatos fájl, vagy túl nagy az egészben való beolvasáshoz
viewer-base64-done = Dekódolva: { $bytes }
viewer-not-base64 = Ez a fájl nem base64: { $why }
viewer-status = { $mode } { $codepage }  { $offset } / { $len }  { $percent }%

key-disasm = Disasm
viewer-disasm-failed = nem fejthető vissza: { $name }: { $error }
viewer-decompile-running = { $tool } fut ezen: { $name }…
viewer-decompile-none = itt semmi sem tudja visszafejteni ezt: { $format }; telepítsen egy eszközt, és adjon [[decompile]] szabályt az fcmd.toml fájlhoz
arc-needs-program = ezt az archívumot a(z) `{ $program }` olvassa, amely nincs telepítve ezen a gépen
viewer-decompile-failed = { $tool }: { $error }
viewer-blame-derived = a git blame magára a fájlra vonatkozik: előbb térjen vissza hozzá
viewer-derived = { $producer } — { $name }

# Editor (F4)
key-save = Mentés
key-replace = Csere
editor-prompt-goto = Ugrás sorra vagy sor:oszlop :
editor-prompt-save-as = Fájl mentése, mint:
editor-saved = { $name }: mentve, { $bytes } bájt
editor-nothing-to-save = { $name }: nincs mentetlen változás
editor-not-found = Nem találtam: `{ $query }`
editor-replaced = { $count } csere
editor-read-only = { $name } csak olvasható
editor-readonly-on = csak olvasható: a szerkesztés elutasítva
editor-readonly-off = csak olvasható mód kikapcsolva
editor-no-undo = nincs mit visszavonni
editor-no-redo = nincs mit újra végrehajtani
editor-discard-title = Mentetlen változások
editor-discard-body = { $name } mentetlen változásokat tartalmaz. Kilép, és elveszíti őket?
editor-changed-title = Megváltozott a lemezen
editor-changed-body = { $name } megváltozott a lemezen a megnyitás óta. Felülírja?
editor-reloaded = { $name } újratöltve a lemezről
editor-failed = { $name }: { $error }

# netbox (SPEC 04 §6)
net-sites-title = Kapcsolatok
net-sites-hint = Enter csatlakozás · n URL · Ins tárol · F4 szerkeszt · d bont · Esc bezár
net-no-sites = Még nincs kiszolgáló. Nyomja meg az `n` billentyűt egy URL beírásához, vagy adjon [[site]] blokkokat a konfigurációhoz.
net-not-a-stored-session = ez a sor nyitott kapcsolat, nem tárolt munkamenet
net-site-forgotten = { $name } már nincs tárolva
net-prompt-url = Csatlakozás ide:
net-prompt-password = Jelszó ({ $user }@{ $host }):
net-prompt-passphrase = Jelmondat a kulcshoz ({ $user }@{ $host }):
net-connecting = csatlakozás: { $host }…
net-connected = csatlakozva: { $host }
net-disconnected = { $host } leválasztva
net-not-connected = { $host } nincs csatlakoztatva
net-local-panel = ez a panel helyi fájlrendszeren áll
net-nothing-pending = erre nem vár kapcsolat
net-hostkey-title = Ismeretlen gépkulcs
net-hostkey-body = { $host } nincs a known_hosts fájlban. Az ujjlenyomata:
net-hostkey-question = Feljegyzi, és csatlakozik?
net-hostkey-changed-title = A gépkulcs MEGVÁLTOZOTT
net-hostkey-changed-body = { $host } olyan kulccsal válaszolt, amely nem egyezik a feljegyzettel.
net-hostkey-changed-where = A feljegyzett itt van: { $path }, { $line }. sor.
net-hostkey-changed-what = Egy újratelepített kiszolgáló pontosan úgy néz ki, mint egy közbeékelődő támadó. Itt semmit sem fogadunk el: nyissa meg a fájlt, olvassa el a sort, és törölje saját kezűleg, ha tudja, miért változott.
btn-open-file = Fájl megnyitása
net-fetching = letöltés: { $name }…
net-viewing-copy = { $name }: helyi másolat, a nézőke számára letöltve
net-too-big = { $name } nagyobb, mint { $limit }: a letöltéshez növelje a net.fetch_limit_mb értékét
net-editing = { $name }: helyi másolat szerkesztése; mentéskor visszaíródik
net-writing-back = { $name } visszaírása…
net-written-back = { $name }: visszaírva, { $bytes } bájt
net-write-back-failed = a visszaírás nem sikerült: { $error } — a szerkesztései itt vannak: { $local }
net-changed-title = Megváltozott a kiszolgálón
net-changed-body = { $name } megváltozott a kiszolgálón a letöltés óta. Felülírja?
confirm-delete-permanent = Ez végleges törlés: ezekhez nincs Lomtár.
confirm-trash-title = Áthelyezés a Lomtárba
confirm-trash-body =
    { $count ->
        [one] A Lomtárba akarja dobni: { $name } ({ $bytes })?
       *[other] A Lomtárba akar dobni { $count } elemet ({ $bytes })?
    }
confirm-trash-where = Oda kerülnek, ahol az asztali környezet a törölt fájlokat tartja, és onnan visszaállíthatók.

# ---------------------------------------------------------------- menu bar (SPEC 10)
menu-left = &Bal
menu-right = &Jobb
menu-files = &Fájlok
menu-commands = &Parancsok
menu-options = B&eállítások
menu-brief = &Rövid
menu-medium = &Közepes
menu-full = &Teljes
menu-detailed = Rész&letes
menu-owners = Fájl tula&jdonos
menu-links = Fájl li&nkek
menu-alt-full = Teljes, &pontos méretekkel
menu-descriptions = Fájl&megjegyzések
menu-long-descriptions = &Hosszú megjegyzés
menu-wide = &Széles
menu-info-panel = &Info panel
menu-quick-view = &Gyorsnézet
menu-tree-panel = &Fastruktúra
menu-tab-new = &Új fül
menu-tab-close = Fül bezárása
menu-tab-next = K&övetkező fül
menu-tab-prev = &Előző fül
menu-tab-move = Fül &a másik panelre
menu-tab-lock = Fül &zárolása a mappájához
menu-tab-name = A fül elnevezése
menu-tab-list = Minden fül
menu-sort-modes = Ren&dezési elv
menu-hidden = Rejtett és rendszerfájlok
menu-filter = Fájlsz&űrő
menu-filter-sets = Elnevezett szűrők
menu-panel-onoff = Panel &be/ki
menu-reread = Friss&ítés
menu-change-drive = Meghajtó&váltás
menu-view = &Megnéz
menu-edit = &Szerkeszt
menu-edit-new = Új &fájl szerkesztése
menu-copy = Más&ol
menu-rename = Át&nevez-Mozgat
menu-link = &Link létrehozása
menu-multirename = Átnevezés minta alapján…
menu-multirename-undo = Az utolsó csoportos átnevezés visszavonása
menu-mkdir = Ú&j mappa
menu-delete = &Töröl
menu-wipe = &Kisöpör
menu-checksum = Ellen&őrző összegek…
menu-checksum-verify = Ellenőrzés lista alapján
menu-split = Fájl darabolása…
menu-combine = Fájlok egyesítése
menu-delete-permanent = Vé&gleges törlés
menu-archive-add = Tömörhöz ho&zzáad
menu-archive-extract = Tömörből ki&bont
menu-archive-commands = Tömörítő &parancsok
menu-archive-test = Archívum tesztelése
menu-attributes = Fájl &attribútumok
menu-apply-command = Parancs &végrehajtása
menu-replace-in-files = Csere fájlokban…
menu-select-group = Csoport k&ijelölése
menu-select-ext = Kijelölés e kiterjesztés szerint
menu-unselect-ext = Jelölés lev&étele e kiterjesztés szerint
menu-unselect-group = Jelölést l&evesz
menu-invert-selection = Jelölést megfor&dít
menu-select-all = Mindent kijelöl
menu-restore-selection = Jel&ölést visszatesz
menu-dirsize = Mappamé&ret
menu-find-file = Fájl&keresés
menu-history = Parancs &előzmények
menu-view-history = Néző és szerkesztő e&lőzmények
menu-folders-history = Ma&ppa előzmények
menu-usage = Leme&zhasználat
menu-jobs = &Feladatok
menu-branch = Ágnézet
menu-bookmarks = Mappa gyorsbillentyűk
menu-swap-panels = Panel&csere
menu-sync-panel = Más&ik panel igazítása
menu-panels-onoff = Panelek &be/ki
menu-compare-folders = Mappák össze&hasonlítása
menu-sync-folders = Mappák szinkr&onizálása
menu-compare-contents = Összehasonlítás ta&rtalom szerint
menu-panelize = Parancs kimenete panelre
menu-view-filtered = Megnézés parancson át
menu-learn-keys = Billentyűk tanítása
menu-tunnel = &Alagút nyitása…
menu-tunnels = Ala&gutak…
tunnel-title = Alagút: [helyi:]gép:port
tunnel-list-title = Alagutak
tunnel-none = nincs nyitott alagút
tunnel-open = alagút megnyitva: { $what }
tunnel-closed = alagút lezárva
tunnel-failed = { $what }: { $why }
tunnel-needs-connection = az alagút kapcsolaton át vezet: előbb nyisson egyet az Alt+N billentyűvel
menu-connections = Kapcsolatok
menu-disconnect = Kapcsolat bontása
menu-user-menu = Felhasználói &menü
menu-associations = Fájl&társítások
menu-folder-shortcuts = Mappa gyorsbillent&yűk
menu-plugin-commands = Pl&ugin parancsok
menu-screens-list = Képer&nyők
menu-macro-record = Makró rögzítése
menu-macro-list = Makrók
menu-task-list = Fut&ó programok
menu-system-settings = &Rendszer beállítások
menu-panel-settings = &Panel beállítások
menu-interface-settings = Kezelő&felület beállítások
menu-languages = N&yelvek
menu-themes = &Témák
menu-confirmations = Meg&erősítések
menu-viewer-settings = &Nézőke beállítások
menu-editor-settings = &Szerkesztő beállítások
menu-codepages = &Kódlapok
menu-sort-groups = Rendezési &csoportok
menu-mask-groups = Fájl&maszkok
menu-settings-all = Minden be&állítás
menu-highlighting = Fájlk&iemelések
menu-plugins-config = Pl&ugin beállítások
menu-help = Súgó
menu-save-setup = &Beállítások mentése
menu-quit = Ki&lépés
opt-natural-sort = Számjegyek számként rendezve
opt-allow-reverse = Fordított rendezés engedélyezve
opt-select-folders = A maszkok mappákat is kijelölnek
cmd-no-network-name = Ezen a lemezen lévő útvonalnak nincs hálózati neve
opt-trash = Törlés a Lomtárba
opt-system-copy = A rendszer másolórutinjának használata
opt-save-history = Előzmények megőrzése a munkamenetek között
opt-history-lifetime = Előzmények élettartama napokban
opt-forever = örökké
opt-fetch-limit = Távoli fájlok méretkorlátja
opt-keepalive = SSH keep-alive
opt-off = ki
opt-show-hidden = Rejtett és rendszerfájlok mutatva
opt-column-titles = Oszlopnevek mutatva
opt-status-line = Állapotsor mutatva
opt-scrollbar = Gördítősáv mutatva
opt-free-space = Szabad lemezterület mutatva
opt-auto-refresh = Panel automatikus frissítése
opt-dirs-first = Mappák előre
opt-selected-first = Kijelöltek előre
opt-reverse = Fordított sorrend
opt-view = Panelnézet
opt-sort = Rendezési elv
opt-quick-search = Gyorskeresés
opt-quick-alt = Alt-tal
opt-quick-plain = betűkkel
opt-key-bar = Funkcióbillentyűk sora mutatva
opt-status-bar = Állapotsáv mutatva
opt-syntax = Az ismert nyelvek színezése
opt-markdown = Markdown olvasásra tördelve
opt-table = .csv és .jsonl megnyitása táblázatként
opt-keymap = Billentyűkiosztás
opt-clock = Óra a sarokban
opt-mouse = Egér kezelése
opt-clipboard = A terminál vigye a vágólapot (OSC 52)
opt-command-screen = A parancs kimenete látszik futás közben
opt-menu-always = A menüsor mindig látszik
opt-save-exit = Beállítások mentése kilépéskor
opt-confirm-delete = Törlés
opt-confirm-overwrite = Felülírás
opt-confirm-exit = Kilépés
opt-confirm-editor = Kilépés a szerkesztőből mentetlen változásokkal
opt-applied = beállítások alkalmazva
opt-saved = mentve ide: { $path }
opt-set-title = { $key }
opt-set = { $key } = { $value }
opt-not-a-toggle = Nem igen-nem beállítás: Enterrel írható be az érték
opt-settings-hint = ↑↓ választ · gépeléssel szűkít · Enter vagy F4 érték · Space vált · Del alapérték · Esc bezár
opt-language-set = nyelv: { $name }
sort-name = Név
sort-ext = Kiterjesztés
sort-size = Méret
sort-mtime = Módosítás ideje
sort-btime = Keletkezés ideje
sort-atime = Hozzáférés ideje
sort-descr = Megjegyzés
sort-owner = Tulajdonos
sort-git = Git állapot
key-sort-descr = Megjgy
key-sort-owner = Tulajd
sort-none = Rendezetlen
sort-set = rendezés: { $name }
btn-ok = OK
pick-hint = ↑↓ választ · gépeléssel szűr · Enter alkalmaz · Esc bezár
pick-empty = (nincs miből választani)
# ------------------------------------------------- the three histories (SPEC 06 §6)
hist-pinned = { $line } rögzítve · most már semmi sem szorítja ki
hist-unpinned = { $line } rögzítése megszűnt
hist-is-pinned = { $line } rögzítve van · az Ins előbb feloldja
hist-clear-body = Törli az előzményekből mindazt, ami nincs rögzítve?
hist-cleared = az előzmények üresek, csak a rögzítettek maradtak
history-hint = ↑↓ választ · Enter alkalmaz · Ins rögzít · Shift+Del egyet · Del mindet · Esc bezár


# ------------------------------------------------- session editor (SPEC 10 §3.5)
site-new = Új kapcsolat
site-edit = Kapcsolat szerkesztése
site-name = Név
site-host = Gép
site-port = Port
site-user = Felhasználó
link-title = Link létrehozása
link-name = A link
link-kind = Fajta
link-kind-relative = szimbolikus, relatív — túléli a fa áthelyezését
link-kind-absolute = szimbolikus, abszolút — túléli a link áthelyezését
link-kind-hard = hardlink — második név ugyanarra a fájlra
menu-colors = S&zínek…
palette-title = A(z) { $theme } téma színei
palette-hint = Enter szín · F2 mentés · Esc bezár
palette-ask = Szín ehhez: { $key }
palette-bad-color = { $value } nem szín (név, 0–255 vagy #rrggbb)
palette-saved = a színek beíródtak a(z) { $theme } témába
palette-unsaved = a színek csak a képernyőn változtak — F2 írja ki őket
opt-viewer-status-line = Felső sor mutatva (név és pozíció)
opt-viewer-scrollbar = Gördítősáv mutatva
opt-viewer-key-bar = Funkcióbillentyűk sora mutatva
opt-viewer-wrap = Hosszú sorok
opt-viewer-wrap-off = nincs tördelés
opt-viewer-wrap-char = tördelés a szélnél
opt-viewer-wrap-word = tördelés szóhatáron
opt-viewer-tab-width = Tabulátor szélessége
opt-viewer-remember = Minden fájl pozíciójának és kódlapjának megjegyzése
opt-editor-tabs = Tabulátorok
opt-editor-tabs-keep = maradnak, ahogy vannak
opt-editor-tabs-typed = a begépeltek kifejtése
opt-editor-tabs-all = mind kifejtve a fájl megnyitásakor
opt-editor-tab-width = Tabulátor szélessége
opt-editor-auto-indent = A fenti sor behúzásának megtartása
opt-editor-whitespace = Szóközök és tabulátorok mutatva
opt-editor-cursor-after = A kurzor a keresett szöveg mögé kerül
opt-editor-beyond-eol = A kurzor túlmehet a sor végén
opt-editor-trim-trailing = Sorvégi szóközök levágása mentéskor
editor-not-a-character = ehhez a billentyűhöz nincs beszúrható karakter
paste-one-line =
    { $count ->
        [one] csak az első sor lett beillesztve · még egy sor kimaradt
       *[other] csak az első sor lett beillesztve · még { $count } sor kimaradt
    }
clip-names =
    { $count ->
        [one] egy név a vágólapon
       *[other] { $count } név a vágólapon
    }
clip-paths =
    { $count ->
        [one] egy útvonal a vágólapon
       *[other] { $count } útvonal a vágólapon
    }
clip-line = a parancssor a vágólapon van
clip-selection = a kijelölt rész a vágólapon van
clip-off = a vágólap ki van kapcsolva — `ui.clipboard` a beállításokban
history-gone = { $name } már nincs meg
hl-new = Új színcsoport
hl-edit = Színcsoport
# ------------------------------------------------- the diff screen (SPEC 05 §3)
diff-title = Különbségek
menu-diff = Fájlkülönbségek megtekinté&se
diff-needs-two = Állítsa a kurzort egy-egy fájlra mindkét panelen
diff-not-here = { $name } nem ezen a lemezen lévő fájl
diff-too-big = { $name } nagyobb, mint { $most }
diff-same = A két fájl azonos
diff-where = { $at }. blokk / { $count } · -{ $removed } +{ $added } sor
diff-no-next = Az utolsó különbség
diff-no-previous = Az első különbség
diff-hint = Alt+↑↓ blokkok · Tab oldal · F5 átvesz · F2 kiír · U egyesített · Shift+F2 patch · W C E mellőz · Esc
diff-hint-git = Alt+↑↓ blokkok · S blokk előkészítése · Tab oldal · F5 átvesz · F2 kiír · Esc bezár
diff-hint-git-staged = Alt+↑↓ blokkok · S blokk kivétele · Tab oldal · F5 átvesz · F2 kiír · Esc bezár
diff-stage-edited = a diffben kiíratlan összefésülés van: előbb F2, aztán S
diff-merged-left = A bal fájl átvette ezt a különbséget
diff-merged-right = A jobb fájl átvette ezt a különbséget
diff-no-hunk-here = A képernyő tetején nincs különbség — Alt+↓ odavisz egyet
diff-nothing-to-write = Semmi sem lett összefésülve
diff-written = { $count } kiírva
diff-unwritten = Egy összefésülés nincs kiírva — F2 kiírja, újabb Esc eldobja
diff-ignore-space-on = Szóközök mellőzve
diff-ignore-space-off = A szóközök újra számítanak
diff-ignore-case-on = Kis- és nagybetű mellőzve
diff-ignore-case-off = A kis- és nagybetű újra számít
diff-ignore-eol-on = Sorvégek mellőzve
diff-ignore-eol-off = A sorvégek újra számítanak

grab-hint = ↑↓←→ mozgás · Shift+↑↓←→ kijelöl · Ctrl+A mindet · Enter másol · Esc bezár
grab-copied = { $rows } képernyősor másolva

# ------------------------------------------------- the code pages (SPEC 05 §1.12)
cp-title = Kódlapok
cp-name-title = Mi legyen ennek a kódlapnak a neve
cp-set = Olvasás így: { $name }
menu-panel-names = Nevek kódlapja…
names-title = A nevek olvasása így
names-system = Ahogy a rendszer adja
names-set = A nevek olvasása: { $name }
names-system-set = A nevek úgy, ahogy a rendszer adja
names-not-here = Ezen a rendszeren a név nem bájtsor, így nincs másik olvasata
cp-marked = { $name } mostantól szerepel az F8 körében
cp-unmarked = { $name } már nem szerepel az F8 körében
cp-hint = ↑↓ választ · Enter így olvas · Ins jelöl az F8-hoz · F4 átnevez · Esc bezár

# ------------------------------------------------- the named mask groups (SPEC 02 §5b)
mg-title = Maszkcsoport
mg-name = Név
mg-mask = Maszkok
mg-hint = Ins csoportot ad, Del elvesz, F4 szerkeszt · a `·` a három beépítettet jelöli
mg-saved = <{ $name }> érvényben van; a Beállítások mentése írja ki
mg-removed = <{ $name }> törölve
mg-bad = <{ $name }>: { $problem }
mg-needs-name = név szóköz, vessző és csúcsos zárójel nélkül
mg-needs-mask = az a csoport, amely semmit sem képvisel, semmire sem illeszkedik
mg-built-in = <{ $name }> a három beépített egyike · F4 más maszkokat ad neki

sg-title = Rendezési csoport
sg-name = Név
sg-mask = Maszkok
sg-place = Hová
sg-place-front = a többi elé
sg-place-back = a többi mögé
sg-hint = Ins csoportot ad, Del elvesz, F4 szerkeszt, Ctrl+↑↓ mozgat
sg-saved = A csoport érvényben van; a Beállítások mentése írja ki
sg-removed = { $name } törölve
sg-bad = { $name }: { $problem }
sg-needs-mask = maszk nélküli csoportba semmi sem kerülne
hl-name = Név
hl-mask = Maszk vagy maszkok
hl-kind = Érvényes
hl-kind-both = fájlokra és mappákra
hl-kind-file = csak fájlokra
hl-kind-dir = csak mappákra
hl-size = Méret (1M-10M, -4K, 1G-)
hl-age = Kor napokban (<=1, >=30, 7-30)
hl-date-field = A kor alapja
hl-attrs = Attribútumok (h r x s b, a - tiltást jelent)
hl-fg = Betűszín (#d7875f, red, file.archive)
hl-bg = Háttér, ugyanígy írva
hl-hint = ↑↓ választ · Enter/F4 szerkeszt · Ins új · Del töröl · Ctrl+↑↓ mozgat · Esc bezár
hl-saved = a csoport érvényben van · Beállítások → Beállítások mentése írja fájlba
hl-removed = { $name } törölve · Beállítások → Beállítások mentése írja fájlba
hl-bad = { $name }: { $problem }
filter-off = Nincs szűrő: minden név
filter-hint = ↑↓ választ · Enter használ · F4 szerkeszt · Ins új · Del töröl · Esc bezár
filter-new = Új szűrő
filter-edit = Szűrő
filter-name-hint = névvel megmarad a szűrők menüben; név nélkül egyszer használatos
filter-applied = szűrő bekapcsolva: { $name }
filter-cleared = szűrő kikapcsolva
filter-saved = { $name } a panelen és a szűrők menüben van · Beállítások → Beállítások mentése írja fájlba
filter-removed = { $name } törölve · Beállítások → Beállítások mentése írja fájlba
editor-no-block = nincs kijelölve semmi
editor-unique = { $count } ismétlődő sor eltávolítva
editor-no-bracket = itt nincs zárójel, vagy nincs párja
editor-tabs-expanded = a tabulátorok { $lines } sorban ki lettek fejtve — a mentés ezt írja ki
site-protocol = Protokoll
site-protocol-sftp = SFTP (SSH felett)
site-protocol-ftp = FTP
site-protocol-ftps = FTP TLS-sel
site-protocol-dav = WebDAV
site-protocol-davs = WebDAV HTTPS felett
site-auth = Hitelesítés
site-auth-agent = SSH-ügynök
site-auth-key = kulcsfájl
site-auth-ask = jelszót kér
site-key-file = Kulcsfájl
site-start-dir = Távoli mappa
site-group = Csoport
site-keepalive = Keep-alive (mp)
site-compress = Tömörítés
site-page-session = Munkamenet
site-page-environment = Környezet
site-page-directories = Mappák
site-page-sftp = SFTP
site-password-note = Jelszót nem tárolunk: csatlakozáskor kérjük el.
site-env-eol = Sorvégek átalakítása (még nincs)
site-env-utf8 = UTF-8 nevek kényszerítése (még nincs)
site-jump = Ugrógép
site-jump-note = Ezen a gépen át érhető el: host, user@host vagy user@host:port. Üresen: közvetlenül a kiszolgálóra.
site-sftp-ciphers = Titkosítók kiválasztása (még nincs)
btn-connect = Csatlakozás
site-needs-host = a kapcsolathoz gép kell
site-needs-key = válasszon kulcsfájlt vagy más hitelesítési módot
site-bad-port = `{ $port }` nem szám
site-stored = kapcsolat tárolva: { $name } (Shift+F9 megőrzi)
# ------------------------------------------------- structure pane (SPEC 05 §1.8)
key-struct = Strukt
key-markdown = MD
key-table = Tábla
key-filter = Szűrő
key-decompile = Dekomp
key-next = TovKer
key-prev = VisKer
key-codepages = Kódlap
key-goto = Ugrás
key-to-panel = Panel
key-blame = Blame
viewer-structure = Szerkezet
viewer-structure-hint = ↑↓ sor · Enter odaugrik · Tab tartalom · Shift+F11 bezár
viewer-structure-hint-tab = A Tab ide hozza a billentyűket
viewer-no-structure = nincs mit dekódolni: ezt a formátumot a szerkezetpanel nem ismeri
viewer-no-target = ez a sor sehová sem mutat a fájlban
viewer-form-opened = Az űrlap abban a szöveges alakban, ahogy a Delphi mutatja. F4 visszavisz a bájtokhoz.
viewer-metadata-opened = A teljes szöveg, olvasásra tördelve. F4 visszavisz a bájtokhoz.
viewer-went-to = ugrás ide: { $offset }
viewer-in-panel = a panel ezen áll: { $name }
viewer-not-in-panel = { $name } nincs ezen a panelen
viewer-no-more-files = nincs több fájl a panelen
editor-unsaved-for-viewer = előbb mentsen: a nézőke a lemezen lévő fájlt olvassa
# ------------------------------------------------- command line (SPEC 06 §7)
cmd-running = fut: { $command }
prompt-panelize = Parancs, amelynek kimenete a fájlokat nevezi meg
prompt-viewer-filter = Parancs, amelynek kimenetét a fájl helyett olvassuk
learn-title = Milyen billentyűket küld ez a terminál
learn-hint = Nyomja le a billentyűket. A kivilágított cella megérkezett; két Esc kilép.
learn-last = utolsó: { $key } — { $command }
learn-unbound = nincs semmihez kötve
recode-title = Újraírás így
recode-what = { $count } fájl
recode-from = Jelenleg mik
recode-to = Erre írandók
recode-auto = fájlonként kiderítve
recode-eol = Sorvégek
recode-eol-keep = maradnak
recode-working = { $count } fájl újraírása
recode-done = { $changed } megírva, { $same } már megfelelt, { $failed } elutasítva
menu-recode = Újraírás így…
panelize-title = { $command }
panelize-done = { $count } fájl, { $dropped } sor kihagyva
panelize-nothing = { $command }: egyetlen sor sem nevezett meg létező fájlt
cmd-interrupted = megszakítás elküldve a parancsnak
cmd-done = { $command } befejeződött
cmd-failed = { $command } kilépési kódja: { $code }
cmd-busy = már fut egy parancs
shell-not-kept = a(z) { $shell } nem jelölte időben a promptját: minden parancs mostantól külön fut, mint korábban
prompt-apply = Parancs, amely mind a(z) { $count } fájlon lefut:
cmd-apply-no-ask = értéket kérő parancs nem alkalmazható fájlonként
cmd-apply-progress = { $command }  ({ $done } / { $total })
cmd-apply-stopped = leállt { $done } / { $total } után: { $command } hibát adott
cmd-not-remote = a parancshoz helyi mappa kell; ez a panel kiszolgálón áll
cmd-no-history = a parancs előzmények üresek
cmd-no-previous = nincs hová visszalépni
cmd-no-such-dir = { $path }: nincs ilyen mappa
cmd-no-such-path = { $path }: nincs ilyen fájl vagy mappa
cmd-no-selection = nincs kijelölve semmi
cmd-condition-false = nem futott le: { $command }
cmd-screen-cleared = képernyő törölve
cmd-stack-empty = a mappaverem üres
cmd-stack-cleared = mappaverem kiürítve
cmd-pushed = { $count } mappa a veremben
cmd-environment = környezet
cmd-set = { $name } = { $value }
cmd-not-set = { $name } nincs beállítva
user-screen-title = Kimenet — { $lines } sor, a(z) { $at }. sortól
user-screen-empty = (még semmi sem futott)
viewer-no-branch = ennek a sornak nincs követhető címe
viewer-no-back = nincs hová visszalépni

# ------------------------------------------------- find files (SPEC 06 §1)
folders-title = Mappakeresés
folders-reading = Mappák olvasása…
folders-read = { $count } mappa
folders-capped = { $count } mappa, a korlátig
folders-local-only = a mappalista erről a gépről szól; a kiszolgálónak előzménye és gyorslistája van
find-title = Fájlkeresés
find-mask = Fájlmaszk(ok)
find-size = Méret (100M, 1M-1G, -4K)
find-text = Tartalmazott szöveg
find-case = Nagy/kisbetű érzékeny
find-words = Csak egész szavak
find-hex = A fenti szöveg hexa minta
find-not = Azok a fájlok, amelyek NEM tartalmazzák
find-scope = Keresés hatósugara
find-scope-panel = csak az aktuális mappában
find-scope-tree = az aktuális mappától
find-scope-selection = a kijelölt mappákban
find-scope-root = a meghajtó gyökerétől
find-scope-other = a másik paneltől lefelé
find-links = Keresés szimbolikus linkekben
find-dirs = Keresés mappákra
find-archives = Keresés tömörítettekben
find-results = { $count } találat · { $files } fájl és { $dirs } mappa átnézve
find-done = { $count } találat { $files } fájlban · { $elapsed }
find-none = nincs találat
find-stopped = leállítva { $files } fájl után
find-hint = ↑↓ választ · Enter odaugrik · F3 megnéz · F4 szerkeszt · F5 panelre · Esc állj
find-hint-done = ↑↓ választ · Enter odaugrik · F3 megnéz · F4 szerkeszt · F5 panelre · Esc bezár
find-panelised = { $count } találat a panelen
find-needs-mask = maszk kell: a `*` minden fájlt jelent
find-bad-size = `{ $text }` nem méret — { $error }

# ------------------------------------------------- replace across files
replace-title = Csere fájlokban
replace-needle = Keresett szöveg
replace-with = Csere erre
replace-case = Nagy/kisbetű érzékeny
replace-words = Csak egész szavak
replace-pattern = A fenti szöveg reguláris kifejezés
replace-hex = Mindkettő hexa bájtsor
replace-scope = { $count } fájl kijelölve
replace-needs-needle = nincs mit keresni
replace-confirm-title = Csere
replace-confirm-body = { $count } előfordulás változna { $files } fájlban.
replace-nothing = nincs mit cserélni: egyetlen fájl sem változna
replace-done = { $count } előfordulás cserélve { $files } fájlban

# ── archives (SPEC 04 §4) ───────────────────────────────────────────────────────────────
arc-writing-back = { $name } visszaírása az archívumba
arc-move-out = archívumból kivenni F5-tel, másolással lehet
arc-add-files-only = egyenként csak fájlok kerülnek be; a Shift+F1 egész mappát tesz be

rename-title = Csoportos átnevezés
rename-name = Név
rename-ext = Kiterjesztés
rename-search = Keresett szöveg
rename-replace = Csere erre
rename-regex = Reguláris kifejezés ($1 a cserében)
rename-case = Betűk
rename-case-same = ahogy vannak
rename-case-lower = kisbetűs
rename-case-upper = NAGYBETŰS
rename-case-title = Minden Szó Nagy Kezdőbetűvel
rename-case-sentence = Csak az első betű nagy
rename-counter = Számláló: kezdet, lépés, számjegyek
rename-fields = [N] név  [E] kiterjesztés  [N1-3] rész  [C] számláló  [d] [t] dátum, idő  [P] mappa
rename-not-in-archive = több fájl mappában nevezhető át, nem archívumban: ott az F6 egy bejegyzést nevez át
rename-bad-counter = a számláló három egész szám: kezdet, lépés, számjegyek; { $counter } nem az
rename-cannot = nem nevezhető át: { $why }
rename-problem-empty = nem maradt név
rename-problem-invalid = fájlnak nem lehet ilyen neve
rename-problem-twice = egy másik fájl is ezt a nevet kapná
rename-problem-exists = ilyen nevű fájl már van
rename-more = … és még { $count }
rename-blocked =
    { $count ->
        [one] egy fájl nem kaphatja meg az új nevét: amíg nem, semmi sem lesz átnevezve
       *[other] { $count } fájl nem kaphatja meg az új nevét: amíg nem, semmi sem lesz átnevezve
    }
rename-summary = { $count } változik a(z) { $total } közül
rename-nothing = egyetlen név sem változik
rename-nothing-to-undo = nincs visszavonható csoportos átnevezés
rename-done =
    { $count ->
        [one] egy fájl átnevezve · visszavonható a Fájlok menüből
       *[other] { $count } fájl átnevezve · visszavonható a Fájlok menüből
    }
rename-undone =
    { $count ->
        [one] egy név visszaállítva
       *[other] { $count } név visszaállítva
    }
pack-title = Tömörhöz hozzáad
pack-name = Archívum
pack-format = Formátum
pack-level = Tömörítés
pack-solid = Szolid blokkok (7z: kisebb, de egy fájl lassabban olvasható)
pack-delete-after = Fájlok törlése tömörítés után
pack-level-normal = normál
pack-level-fast = gyors
pack-level-best = legjobb
pack-scope = { $count } fájl kijelölve
pack-needs-name = az archívumnak név kell
pack-format-read-only = a(z) { $format } itt csak olvasható, nem írható: válasszon más formátumot vagy más nevet
pack-single-one-file = egy { $format } egyetlen fájlt tartalmaz: válasszon egyet, vagy mappához, több fájlhoz ezt: { $tar }
unpack-title = Tömörből kibont
unpack-to = Kibontás ide
unpack-scope = innen: { $name }
unpack-not-an-archive = nem olyan archívum, amelyet olvasni tudunk

# ── the special panel kinds (SPEC 02 §7) ────────────────────────────────────────────────
panel-quick-view = Gyorsnézet
panel-info = Információk
panel-tree = Fa
compare-done = balra { $left }, jobbra { $right } eltérő
cmp-subdirs = Belépés az almappákba
cmp-depth = Milyen mélyre (0 = egészen le)
cmp-trees-title = Mi tér el
cmp-trees-working = fák összehasonlítása: { $seen } fájl, { $read } beolvasva
cmp-trees-done = { $differ } eltér, { $left } csak balra, { $right } csak jobbra, { $pairs } párból
cmp-trees-same = a két fa azonos ({ $pairs } pár, { $same } ellenőrizve)
menu-duplicates = Duplikátumok keresése
dup-title = Duplikált fájlok
dup-working = duplikátumok keresése: { $seen } fájl, { $hashed } beolvasva
dup-done = { $groups } másolatcsoport, { $files } fájl, { $wasted } pazarolva
dup-none = itt semmi sem másolata semmi másnak ({ $seen } fájl)
dup-stopped = leállítva { $seen } fájl után
dup-partial = idő előtt leállt
cmp-title = Összehasonlítás
cmp-case = A nevek egyeztetése nagy/kisbetű érzékenyen
cmp-size = Az eltérő méret különbség
cmp-time = Az eltérő idő különbség
cmp-seconds = Ennyi másodpercen belüli idők azonosnak számítanak
cmp-contents = A bájtok beolvasása
cmp-when-time = Csak ott olvasson, ahol az idők eltérnek
cmp-fragment = Csak az első ennyi KiB beolvasása (0 = az egész)
cmp-ignore-eol = A sorvégek önmagukban nem jelentenek különbséget
cmp-selected = Csak a kijelöltek
cmp-stop = Megállás az első különbségnél
cmp-newer = Csak az újabb példány kijelölése
menu-compare-setup = Összehasonlítás beáll&ításokkal
compare-contents-eol-ignored = { $count } csak sorvégekben tér el, mellőzve
compare-contents-trusted = { $count } olvasás nélkül azonosnak véve
compare-contents-working = Tartalom összehasonlítása: { $done } / { $total }
compare-contents-done = { $differ } pár tér el a(z) { $pairs } közül
compare-contents-same = mind a(z) { $pairs } pár azonos
compare-contents-none = egyetlen fájlnak sincs azonos neve a két oldalon
compare-contents-local = a tartalom szerinti összehasonlításhoz mindkét panelnek helyi fájlrendszeren kell állnia
compare-contents-eol = { $count } csak sorvégekben
compare-contents-only = { $count } csak az egyik oldalon
compare-contents-unread = { $count } nem volt olvasható
compare-contents-stopped = összehasonlítás leállítva
quick-view-directory = { $name } mappa — az F3 megméri
quick-view-not-here = a bájtok nincsenek itt: az F3 előbb idehozza őket
info-path = Útvonal
info-kind = Fajta
info-kind-local = mappa ezen a gépen
info-kind-remote = itt: { $host }
info-kind-archive = ebben: { $name }
info-kind-list = fájllista
info-device = Eszköz
info-folder = Mappa
info-device-value = { $total }, { $free } szabad, { $cluster } klaszter
info-slack = Meddő terület
info-filesystem = Fájlrendszer
info-mount = Csatolva
info-git = Tároló
info-git-clean = egyezik az indexszel
info-git-differing = { $count } tér el az indextől a(z) { $tracked } közül
info-git-uncounted = nincs megszámolva
info-git-merge = merge folyamatban
info-git-rebase = rebase folyamatban
info-git-cherry-pick = cherry-pick folyamatban
info-git-revert = revert folyamatban
info-git-bisect = bisect folyamatban
info-memory = Memória
info-memory-value = { $free } szabad, összesen { $total }
info-memory-total = { $total }
info-contents = Tartalom
info-contents-value = { $files } fájl, { $dirs } mappa, { $bytes }
info-selected = Kijelölve
info-selected-value = { $count } fájl, { $bytes }
info-cursor = Kurzor
info-size = Méret
info-modified = Módosítva
info-description = Megjegyzés
key-mark = Jelöl
key-asm = Asm
key-block = Blokk
key-screens = Képrny
key-site-edit = SzrkMm
key-site-new = ÚjMm
key-site-forget = TörlMm
key-ai-edit = Szerk.
key-ai-new = Új
key-ai-remove = Töröl
key-ai-forget = Felejt
key-ai-threads = Előzm
hex-needs-a-file = ennek a puffernek nincs fájlja, amelyből bájtok olvashatók
hex-save-first = előbb mentsen: a két szerkesztő nem tarthatja egymás mentetlen változásait

# ── the hex editor's block operations (SPEC 05 §2.6) ────────────────────────────────────
hex-block-title = Blokk
hex-op-write = A(z) { $size } kiírása fájlba
hex-op-read = Fájl beolvasása ide: { $at }
hex-op-insert = Nulla bájtok beszúrása ide: { $at }
hex-op-copy = Bájtok másolása máshonnan ide: { $at }
hex-op-move = Bájtok mozgatása máshonnan ide: { $at }
hex-op-fill = A(z) { $size } kitöltése egy bájttal
hex-op-xor = A(z) { $size } kizáró-vagy művelete egy kulccsal
hex-op-delete = A(z) { $size } törlése — { $was } helyett { $becomes } lesz
hex-write-title = A blokk kiírása ide
hex-read-title = Melyik fájl beolvasása
hex-fill-title = Melyik bájttal töltsön (hexa)
hex-xor-title = Kizáró-vagy melyik kulccsal (hexa)
hex-insert-title = Hány nulla bájt
hex-copy-title = Másolás honnan — eltolás és hossz (hexa)
hex-move-title = Mozgatás honnan — eltolás és hossz (hexa)
hex-not-hex = ez nem hexa számjegyek sora
hex-not-a-count = ez nem bájtszám
hex-not-a-range = ez nem eltolás és hossz hexában
hex-range-outside = ez a tartomány kívül esik a fájlon ({ $size })
hex-move-into-itself = egy blokk nem mozgatható önmagába
hex-filled = { $size } kitöltve
hex-xored = { $size } kizáró-vagy művelettel módosítva — ez nem titkosítás
hex-written = { $size } kiírva ide: { $name }
hex-read-done = { $size } beolvasva innen: { $name }
hex-inserted = { $size } nulla beszúrva
hex-deleted = { $size } törölve
hex-device = { $name }: { $size } méretű eszköz, csak olvasásra megnyitva
asm-title = Assemblálás itt: { $at } ({ $arch })
asm-no-arch = { $arch } nem olyan architektúra, amelyre assemblálni tudunk
asm-written = { $text } — { $bytes }
asm-written-longer = { $text } — { $bytes } ({ $grew } bájttal hosszabb)
screens-title = Képernyők
screens-panels = Panelek
quit-title = Kilépés a FinalCommanderből
quit-body = Biztosan kilép?
screens-quit-title = Mentetlen változások
screens-quit-body = Ezeken a képernyőkön kiíratlan változások vannak:
macro-recording = Makró rögzítése · Ctrl+. leállítja
macro-empty = semmi sem lett rögzítve
macro-dropped = a felvétel eldobva
macro-key-title = Melyik billentyű
macro-key-body = { $steps } lépés rögzítve. Nyomja le a billentyűt, amelyre a makró hallgatni fog, vagy Esc az eldobáshoz.
macro-describe = Mire való a(z) { $key } makrója
macro-saved = a(z) { $key } makrója mentve, { $steps } lépés
macro-none = még nincs rögzített makró · Ctrl+. rögzít egyet
macro-list-title = Makrók
macro-play-title = Hogyan fusson
macro-play-once = Egyszer, a kurzor helyétől
macro-play-each = Egyszer minden kijelölt fájlra
macro-sure-title = A makró lejátszása
macro-sure-body = A(z) { $key } makrója másol, mozgat, töröl vagy felülír. Lejátssza?
macro-too-deep = egy makró nem játszhatja le önmagát a végtelenségig
plugin-list-title = Plugin parancsok
plugin-none-title = Nincs telepített plugin
plugin-none-open = A pluginok mappájának megnyitása ({ $dir })
plugin-none-help = Hogyan működnek a pluginok, és hol találhatók
plugin-help-heading = Pluginok
plugin-why-off = kikapcsolva itt: Beállítások → Plugin beállítások
plugin-why-panel = csak a paneleken
plugin-why-viewer = csak a nézőkében
plugin-why-editor = csak a szerkesztőben
plugin-why-local = e gép lemezein lévő mappa kell hozzá
plugin-why-files = kurzor alatti vagy kijelölt fájl kell hozzá
plugin-why-masks = csak ezekre: { $masks }
plugin-why-bad-mask = hibás a maszkja: { $error }
plugin-why-here = ez kell hozzá ebben a mappában: { $names }
plugin-why-inside = ez kell hozzá itt vagy egy feljebb lévő mappában: { $names }
plugin-cannot = { $label }: { $why }
plugin-running = { $label }…
plugin-on-selected = { $count } kijelölt fájlon
plugin-failed = { $id }: { $why }
plugin-unknown-command = egy plugin ezt kérte: `{ $id }`, ami nem parancs
hex-copied = { $size } átmásolva innen: { $from }
hex-moved = { $size } áthelyezve innen: { $from }
menu-calculator = Számológép
menu-terminal = Terminál
calc-title = Számológép
calc-decimal = decimális
calc-hex = hexa
calc-octal = oktális
calc-binary = bináris
calc-characters = karakterek

# The archive commands (Shift+F3) and the two parts of the order that are not the key.
arc-commands-title = Tömörítő parancsok
arc-password = Jelszó ehhez: { $name }
arc-op-test = { $name } tesztelése
arc-op-extract-here = { $name } kibontása ide
arc-op-extract-to = { $name } kibontása máshová…
arc-op-delete = { $name } törlése
arc-op-add = Fájlok hozzáadása archívumhoz…
job-test = Archívum tesztelése
msg-dirs-first-on = mappák előre
msg-dirs-first-off = mappák a fájlok között
msg-selected-first-on = kijelöltek előre
msg-selected-first-off = kijelöltek a sorrendben

# The change-drive dialog (Alt+F1, Alt+F2): the type column, and its footer.
drive-fixed = Fix
drive-removable = Kivehető
drive-optical = Optikai
drive-network = Hálózati
drive-memory = Memória
drive-home = Saját mappa
drive-connection = Kapcsolat
drive-objects = Objektumtár
drive-netbox = NetBox: tárolt munkamenetek
drive-processes = Folyamatlista
drive-temp = Ideiglenes panel
temp-added = { $count } hozzáadva az ideiglenes panelhez
proc-none = a futó folyamatok listája itt nem olvasható
drive-programs = Telepített programok
drive-services = Szolgáltatások
svc-none = ezen a gépen nincs megkérdezhető szolgáltatáskezelő
temp-not-a-file = az itteni sorok nem fájlok: nincs mit törölni
svc-card = Szolgáltatás
svc-card-id = Név
svc-card-name = Megjelenített név
svc-card-state = Állapot
svc-card-description = Leírás
svc-card-pid = Folyamat
svc-state-running = fut
svc-state-stopped = leállítva
svc-state-failed = hibás
svc-state-other = egyéb
key-service-start = Indít
key-service-restart = Újraind
key-service-stop = Leállít
svc-ask-stop = Leállítja ezt: { $name }?
svc-ask-restart = Újraindítja ezt: { $name }?
btn-service-stop = Leállít
btn-service-restart = Újraindít
svc-working = { $name }: a rendszer megkérdezése…
svc-started = { $name } elindítva
svc-stopped = { $name } leállítva
svc-restarted = { $name } újraindítva
svc-needs-root = { $name }: ehhez root jog kell — a program nem kéri el Ön helyett
svc-failed = { $name }: { $error }
prog-none = itt semmi sem mondja meg, mi van telepítve
prog-card = Telepített program
prog-no-uninstall = semmi sem mondja meg, hogyan távolítható el: { $name }
prog-uninstall-ready = a(z) { $name } eltávolító parancsa a parancssorban van, futtatás nélkül
prog-card-name = Név
prog-card-version = Verzió
prog-card-publisher = Kiadó
prog-card-size = Méret a lemezen
prog-card-installed = Telepítve
prog-card-location = Mappa
prog-card-uninstall = Eltávolítás
drive-sockets = Socketlista
sock-none = itt semmi sem figyel, vagy a tábla nem olvasható
sock-no-process = semmi sem mondja meg, melyik folyamat tartja ezt a socketet
proc-card = Folyamat
proc-gone = { $name } már nem fut
proc-card-name = Név
proc-card-pid = Folyamatazonosító
proc-card-parent = Indította
proc-card-user = Felhasználó
proc-card-state = Állapot
proc-card-threads = Szálak
proc-card-memory = Memória
proc-card-mapped = Leképezve
proc-card-bits = Bitszám
proc-card-started = Elindítva
proc-card-exe = Futtatható fájl
proc-card-cwd = Mappa
proc-card-command = Parancssor
proc-card-group = Csoport
proc-card-sid = SID
proc-card-priority = Prioritás
proc-card-session = Munkamenet
proc-card-uptime = Ennyi ideje fut
proc-card-cpu-user = Processzoridő, saját
proc-card-cpu-kernel = Processzoridő, a kernelben
proc-card-handles = Nyitott leírók
proc-card-faults = Laphibák
proc-card-private = Privát memória
proc-card-swap = Kilapozva
proc-card-io-read = Olvasva
proc-card-io-write = Írva
proc-card-memory-peak = Munkakészlet csúcsa
proc-card-private-resident = Munkakészlet, privát
proc-card-swap-peak = Lapozófájl csúcsa
proc-card-virtual = Virtuális bájtok
proc-card-virtual-peak = Virtuális bájtok csúcsa
proc-card-paged-pool = Lapozható készlet
proc-card-nonpaged-pool = Nem lapozható készlet
proc-card-io-other = Egyéb I/O
proc-card-io-read-ops = Olvasási műveletek
proc-card-io-write-ops = Írási műveletek
proc-card-io-other-ops = Egyéb műveletek
proc-kill-title = Folyamat befejezése
proc-kill-ask = Befejezi ezt: { $name } [{ $pid }]? Amit nem mentett el, elvész.
proc-killing = { $name } befejezése…
proc-killed = { $name } befejezve
proc-kill-failed = { $name } nem volt befejezhető: { $why }
proc-card-env = Környezet
proc-card-modules = Betöltött modulok
proc-card-version = Verzió
proc-card-description = Leírás
drives-hint = ↑↓ választ · 0-9 panelek · Ctrl+1..9 oszlopok · F9 mind · Ctrl+R frissít
drives-title = Meghajtók: mi látsszon
drives-col-type = a típus
drives-col-device = az eszköz
drives-col-label = a kötetcímke
drives-col-fs = a fájlrendszer
drives-col-size = a méretek
drives-col-removable = a kivehető meghajtók
drives-col-panels = a nem meghajtó panelek
drives-col-optical = az optikai meghajtók
drives-col-network = a hálózati megosztások
drives-shown = mutatva: { $what }
drives-hidden = elrejtve: { $what }
drives-reread = a meghajtók újra beolvasva
bookmarks-hint = Enter odalép · Ins megjegyez · Del töröl · F4 név · Esc bezár

# The search dialog (F7), which asks the whole question in one place.
search-title = Keresés
search-for = Keresett szöveg
search-mode = Mint
search-mode-text = szöveg
search-mode-hex = hexa
search-mode-regex = reguláris kifejezés
search-case = Nagy/kisbetű érzékeny
search-words = Csak egész szavak
btn-find-next = Következő
btn-find-prev = Előző
# The help: the chapters' own names, and the note on a chapter nobody has translated yet
help-chapter-about = Mi ez
help-chapter-panels = A panelek
help-chapter-files = Fájlok: másolás, mozgatás, keresés, archívumok
help-chapter-viewer = A nézőke
help-chapter-editor = A szerkesztő, a hexa panel és az assembler
help-chapter-network = Kiszolgálók
help-chapter-config = Konfiguráció, billentyűk és színek
help-chapter-keys = A billentyűk, ahogy most ki vannak osztva
help-chapter-commands = Minden parancs és a billentyűi
help-chapter-options = Minden beállítás és az értéke
help-here-panels = A panelek
help-here-viewer = A nézőke
help-here-editor = A szerkesztő
help-here-hex = A hexa szerkesztő
help-here-dialog = Ez a párbeszédablak
help-in-english = Ezt a fejezetet még nem fordították le az Ön nyelvére, ezért angolul következik. A benne említett billentyűk és beállítások minden nyelven ugyanazok.
help-about-byline = FinalCommander { $version } — írta Matteo Baccan
key-ok = OK
key-cancel = Mégsem
key-dialog-move = Mozgat
key-tree = Fa
key-config = Konfig
browse-local-only = fájl erről a gépről választható; kiszolgálón lévő útvonalat be kell írni, a Tab kiegészíti
key-browse = Tallóz
table-prompt-filter = Sorok, amelyek tartalmazzák
table-sorted = rendezve { $column } szerint
table-sorted-back = rendezve { $column } szerint, fordítva
table-unsorted = vissza a fájl szerinti sorrendhez
table-filtered = { $rows } sor maradt
table-unfiltered = újra minden sor
table-cut-sorted = az első { $rows } sorból, mert csak ennyi lett beolvasva
table-too-many = túl sok rekord az itteni rendezéshez vagy szűréshez
table-copied =
    { $rows ->
        [one] egy sor a vágólapon
       *[other] { $rows } sor a vágólapon
    }
table-headed = az első sor a fejléc
table-not-headed = az első sor adat: az oszlopok a számukkal hívhatók
table-named-fields = ez a formátum maga nevezi meg a mezőit: nincs kitalálandó fejléc

# Splitting and combining (SPEC 03 §9)
split-title = Fájl darabolása
split-target = Ide
split-size = Egy darab mérete
split-size-hint = 1440K, 700M, 4G — az utolsó darab a maradék
split-count = Vagy ennyi
split-count-hint = darabra — az itt megadott szám felülírja a méretet
split-bad-count = { $count } nem darabszám: kettő vagy több kell
split-local-file = A darabolás és az egyesítés ezen a gépen lévő fájlon működik
split-bad-size = „{ $size }” nem méret: így írja: 700M vagy 4G
combine-not-a-piece = Ez nem darab: a darabolt fájl darabjai .001, .002, … végűek, vagy .crc tartozik hozzájuk
pieces-working = Darabolás vagy egyesítés: { $done } / { $total }
pieces-cancelled = Leállítva, a kiírt részek visszavonva
split-done = { $parts } darab kiírva, mellettük: { $list }
combine-done-checked = { $name } egyesítve { $parts } darabból, { $bytes }, és egyezik a .crc-vel
combine-done-unchecked = { $name } egyesítve { $parts } darabból, { $bytes } — nem volt .crc az ellenőrzéshez
pieces-failed = Nem sikerült: { $error }

# The branch view (SPEC 02 §3d)
branch-reading = Minden fájl beolvasása itt: { $path }…
branch-title = Ág: { $path }
branch-done = { $count } fájl, az összes lejjebb lévő mappából · Ctrl+Alt+B visszavisz
branch-truncated = { $count } fájl, és a bejárás itt megállt: az ág nagyobb, mint amekkora lista még hasznos
branch-not-here = Az ágnézet mappáról készül, ez a panel pedig lista

## The release check (SPEC 11 §5).
menu-check-update = Friss&ítések keresése
opt-update-check = Új kiadás keresése naponta egyszer
btn-open-page = Oldal megnyitása
update-checking = A legújabb kiadás lekérdezése a GitHubról…
update-title = Frissítések
update-available = Megjelent a FinalCommander { $version } (ez itt: { $current }).
update-available-status = Megjelent a FinalCommander { $version }: Beállítások → Frissítések keresése
update-current = Ez a legújabb kiadás: { $version }.
update-failed = A legújabb kiadás nem volt lekérdezhető: { $why }

## Duplicate files (SPEC 06 §13).
menu-dupes = Duplikált fájlok keresése
dupes-title = Duplikált fájlok
dupes-recursive = Keresés az almappákban
dupes-min-size = Legkisebb fájl
dupes-min-size-hint = Méret, ahogy a szűrők írják: 1, 100K, 5M. Üres fájlokat sosem hasonlítunk össze.
dupes-select = Minden példány kijelölése a legrégebbi kivételével
btn-search = Keresés
dupes-not-here = A duplikátumkeresés mappából vagy a kijelöltekből indul.
dupes-bad-size = Nem méret: { $size }
dupes-looking = Duplikált fájlok keresése…
dupes-progress = Duplikátumok: { $pass }. menet a 3-ból, { $done }/{ $total }
dupes-none = Nincs duplikátum { $count } fájl között.
dupes-panel = Duplikátumok: { $groups } csoport
dupes-done = { $groups } duplikátumcsoport, { $size } a másolatokban.
dupes-truncated = Idő előtt leállt: eddig { $groups } csoport, { $size } a másolatokban.

## SQLite queries (SPEC 04 §4b).
sql-empty = Írjon lekérdezést az sql: után, például sql: SELECT * FROM t
sql-no-database = az sql: parancshoz SQLite adatbázis kell: lépjen be egybe, vagy állítsa rá a kurzort.
sql-running = A lekérdezés fut…
sql-failed = A lekérdezés nem sikerült: { $why }

## Asking a language model (SPEC 06 §14).
ai-none = Nincs beállított AI-szolgáltatás: adjon [[ai]] blokkot az fcmd.toml fájlhoz (name, url, model, api, token_env).
ai-title = Kérdés az AI-hoz
ai-no-thread = nincs folytatható válasz: előbb kérdezzen az `ai:` paranccsal, vagy állítsa a kurzort egy válaszra a válaszok mappájában
ai-nothing-to-explain = itt nincs mit elmagyarázni: nyisson meg egy fájlt a nézőkében vagy a szerkesztőben, vagy kérdezzen közvetlenül egy hibás parancs után
ai-explain-question = Mi ez, innen: { $name }? Magyarázd el röviden.
ai-tokens = { $in } token be, { $out } ki
ai-none-panel = Még nincs AI-szolgáltatás — az F7 hozzáad egyet
ai-service-removed = { $name } eltávolítva
ai-probing = Bejelentkezés: { $name }, { $host }…
ai-probe-ok = { $host } HTTP { $status } választ adott { $ms } ms alatt, { $models } modell — írjon be egy kérdést, és nyomjon Entert
ai-probe-no-model = (a(z) { $model } modell nincs köztük)
ai-row-context = kontextus
ai-row-conversation = beszélgetés
ai-session-title = AI: { $name } · { $model } — { $files } fájl a kontextusban
ai-context-added = { $count ->
        [one] Egy fájl
       *[other] { $count } fájl
    } a kontextusban
ai-context-nothing = Nincs mit hozzáadni: a kontextusba csak ezen a lemezen lévő fájlok kerülnek
ai-command-ready = a parancs a sorban van — olvassa el, mielőtt Entert nyom
ai-commit-question = Írj commit üzenetet az előkészített változásokhoz.
ai-staged-diff = az előkészített diff
ai-send-diff = Az előkészített diff ide megy: { $host }
ai-send-files = { $count } fájl, { $size } megy a kérdéssel együtt ide: { $host }.
ai-asking = Kérdés ehhez: { $name }…
ai-busy = { $name } gondolkodik… { $secs } mp · Esc leállítja
ai-conversation-new = Új beszélgetés: a következő kérdés indítja el.
ai-live-empty = Írjon kérdést a parancssorba: ide megy: { $name }, a beszélgetés pedig menet közben itt látszik.
ai-failed = Az AI nem válaszolt: { $why }
ai-stopped = leállítva: a(z) { $name } felé küldött kérdés eldobva, semmi sem íródott ki
ai-context-title = Fájlok a kontextusban
ai-context-all = Mindegyik
ai-context-dropped = { $count ->
        [one] egy fájl kikerült a kontextusból
       *[other] { $count } fájl kikerült a kontextusból
    }
ai-context-hint = Enter vagy Del: kiveszi a kontextusból — a fájl a lemezen marad · F3: mi megy el · Esc
ai-threads-title = Beszélgetések
ai-threads-hint = Enter: a beszélgetés folytatása · Esc
ai-thread-resumed = folytatás: { $name }
ai-next-estimate = következő ≈ { $tokens }
ai-next-over = következő ≈ { $tokens }, több, mint amennyit a modell elbír
ai-send-estimate = Összesen kb. { $tokens } token, az eddigi beszélgetéssel együtt.
ai-send-estimate-of = Összesen kb. { $tokens } token, az eddigi beszélgetéssel együtt, a modell { $max } tokenes keretéből.
ai-send-over = Összesen kb. { $tokens } token: több, mint a modell { $max } tokenes kerete, így az eleje levágódhat. Az F8 kivesz egy fájlt.
ai-send-question = A kérdés ide megy: { $host }.
btn-send = Küldés

## Git (SPEC 06 §15).
menu-git = Git
git-title = Git
git-status = Mi változott
git-log = A napló
git-blame = Blame: { $name }
git-branches = Ágak
git-local-only = a git ezen a lemezen lévő mappán dolgozik.
git-running = A git megkérdezése…
git-failed = git: { $why }
git-clean = Semmi sem változott.
git-status-title = Változott: { $staged } előkészítve, { $unstaged } nem
git-side-conflict = ütközés
git-side-staged = előkészítve
git-side-changed = nincs előkészítve
git-side-untracked = nem követett
git-kind-modified = módosítva
git-kind-added = új
git-kind-deleted = törölve
git-kind-renamed = átnevezve
git-kind-copied = másolva
git-kind-type = típusa változott
git-kind-conflict = feloldandó
git-as-tree = A változások mutatása faként
git-as-list = A változások mutatása listaként
git-version-head = utolsó commit
git-version-index = előkészítve
git-version-disk = a lemezen
git-discard-title = Változások elvetése
git-discard-body = Elveti { $count ->
        [one] egy fájl
       *[other] { $count } fájl
    } változásait?
git-discard-worktree = vissza az indexhez:
git-discard-both = vissza az utolsó commithoz:
git-discard-added = ki az indexből, a lemezen marad:
git-discard-trash = a Lomtárba:
git-discard-warning = Ami visszaáll az indexre vagy az utolsó commitra, az nem vonható vissza.
git-hint = Enter: választ · Esc: bezár

## The editor's character list and word completion.
charmap-title = Karakterek
charmap-hint = Gépeléssel szűkít (karakter vagy kódja, U+00E9) · Enter: beszúr · Esc: vissza
editor-no-completion = A fájlban nincs más szó, amely így kezdődik.

## A stored session's address, copied (NetBox's Generate URL).
net-url-copied = Másolva: { $url }

## The system log (SPEC 06 §16).
menu-syslog = Rendszernapló
syslog-reading = A rendszernapló olvasása…

## Deleting lines by pattern (FAR #10).
editor-prompt-delete-matching = Az erre illeszkedő sorok törlése (elöl !: csak azok maradnak)
editor-lines-deleted = Törölt sorok: { $count }

## The copy dialog's size policy (FAR #493).
copy-conflict-size = felülír, ha a méret eltér

# Copy dialog: what an error does (FAR's issue #597)
copy-on-error = Ha valamelyik nem másolható
copy-error-ask = Kérdez
copy-error-skip = Kihagyja és folytatja
copy-error-stop = Leállítja a másolást

# Attributes dialog: the times copied from one file to another (FAR's issue #241)
attr-times-copied = Idők másolva: a Ctrl+Shift+V beteszi őket egy másik fájl párbeszédablakába
attr-times-none = Még nincs mit beilleszteni: előbb Ctrl+Shift+C egy fájl párbeszédablakában
attr-times-pasted = Idők beillesztve: az OK beállítja őket

# Editor: replacements kept by name and recent ones (FAR's issue #282)
replace-tpl-title = Cserék
replace-tpl-hint = Enter futtatja · Ins névvel megőriz egy frisset · Del elfelejt egy megőrzöttet · Esc
replace-tpl-name = A csere megőrzése ezen a néven
replace-tpl-kept = Megőrizve: {$name}
replace-tpl-empty = Még nincs csere: előbb Ctrl+F7

# Panel settings: a view and a sort per folder (FAR's issue #389)
opt-remember-view = Minden mappa megőrzi a saját nézetét és rendezését

# Session editor: a cap on a copy's speed to or from the site
site-max-kbps = Sebességkorlát, KB/s (üres: nincs)

# Session editor: IMAP, read only
site-protocol-imap = IMAP (levelek, csak olvasás)
site-protocol-imaps = IMAPS (levelek TLS-en át, csak olvasás)

# The AI services as a list, like the sessions
ai-services-title = AI-szolgáltatások
ai-services-hint = Enter: ezt kérdezi az ai: · Ins új · F4 módosít · Del töröl · Esc
ai-service-title = AI-szolgáltatás
ai-field-name = Név
ai-field-api = Protokoll
ai-field-url = Cím
ai-field-model = Modell
ai-field-token-env = Kulcs ebben a változóban
ai-field-token-note = Üresen: a kulcsot munkamenetenként egyszer kérjük el, és sosem írjuk le.
ai-field-system = Amit elsőként megkap
ai-service-incomplete = A szolgáltatáshoz név és cím kell
ai-service-default = az ai: mostantól ezt kérdezi: {$name}
ai-key-prompt = Kulcs ehhez: {$name} (kilépésig megmarad)
ai-api-openai = OpenAI és kompatibilis
ai-api-anthropic = Anthropic
ai-api-ollama = Ollama (ezen a gépen)
menu-ai = AI-szolgáltatások

# The secrets file
vault-password = Az fcmd titokfájljának jelszava
vault-new = A kulcs megőrzése az fcmd titkosított titokfájljában: válasszon hozzá jelszót (üres: csak erre a munkamenetre)
vault-refused = A titokfájl nem nyílt meg: {$why}

# The programs from elsewhere
drive-tools = Külső programok
drive-ai = AI
drive-git = Git
tools-found = megvan
tools-missing = nincs telepítve
tools-summary = { $found } külső program megvan a(z) { $total } közül; az F3 megmondja, melyik mire való, az F4 telepíti a hiányzót
tools-card = Külső program
tools-card-program = Program
tools-card-where = Hol
tools-card-version = Verzió
tools-card-for = Mire való
tools-card-install = Telepítés
tools-card-why = Miért futtatja az fcmd, ahelyett hogy maga végezné: F1, „Programok máshonnan”.
tools-install-any = telepítse a(z) { $name } programot tartalmazó csomagot, és ügyeljen, hogy a PATH-on legyen
tools-already = { $name } már megvan
tools-for-archives = archívumok: { $what }
tools-for-decompile = Shift+F6 a nézőkében ezen: { $what }
tools-for-git = a Git panel (Ctrl+Alt+G) és az ág a promptban
tools-for-video = videó képkockái a nézőkében és a gyorsnézetben (F3, Ctrl+Q)
tools-for-open = hivatkozás megnyitása a böngészőben
tools-for-syslog = Parancsok → Rendszernapló
tools-for-services = a szolgáltatások panelje
tools-card-site = Webhely
tools-no-package = Ezen a gépen egyetlen ismert csomagkezelőben sincs meg: { $name }. Megnyitja a letöltési oldalát?
tools-install-ask = A(z) { $name } telepítése ezt futtatva egy terminálban:
tools-install-how = A sudo a terminálban kéri el a jelszavát. A Ctrl+Alt+E visszavisz a panelekhez, a Ctrl+R ezen a panelen pedig újra megnézi.
tools-install-else = Ha nem sikerül: { $url }
tools-install-failed = fcmd: { $name } nem lett telepítve. Töltse le innen: { $url }
tools-installing = { $name } telepítése terminálban: a Ctrl+Alt+E visszavisz a panelekhez, ott a Ctrl+R újra megnézi
btn-install = Telepít

# Plugins configuration
plugins-config-title = Pluginok
plugins-config-hint = Enter: be- vagy kikapcsol · F3 mi ez és mit hoz · Esc
plugins-broken = nem töltődött be — az F3 megmondja, miért
plugins-cannot-toggle = a be nem töltődött plugin nem kapcsolható be: az F3 megmondja, mi a baja
plugins-on = { $id } bekapcsolva: a parancsai már ott vannak a Ctrl+Shift+F11 alatt
plugins-on-restart = { $id } bekapcsolva: a parancsai már ott vannak a Ctrl+Shift+F11 alatt, a többit a következő indítástól hozza
plugins-off = { $id } kikapcsolva: a parancsai eltűntek a Ctrl+Shift+F11 alól
plugins-off-restart = { $id } kikapcsolva: a parancsai eltűntek a Ctrl+Shift+F11 alól, a többi a következő indításkor tűnik el
plugins-state-on = be
plugins-state-off = ki
plugins-card-id = Azonosító
plugins-card-name = Név
plugins-card-version = Verzió
plugins-card-author = Szerző
plugins-card-about = Névjegy
plugins-card-dir = Mappa
plugins-card-state = Állapot
plugins-card-brings = Amit hoz
plugins-card-commands = Parancsok (Ctrl+Shift+F11):
plugins-card-notes = Betöltéskor mondta:
plugins-none = nincs plugin · itt laknak: { $dir }, mindegyik külön mappában, egy plugin.toml fájllal

# The git panel
git-file-log = { $name } naplója
git-commit = Az előkészítettek commitolása…
git-pull = Pull
git-push = Push
git-pushing = Push folyamatban…
git-pulling = Pull folyamatban…
git-commit-prompt = A commit üzenete
git-commit-empty = A commithoz üzenet kell.
git-already-on = Már a(z) { $branch } ágon áll.
git-switch-ask = Átvált erre: { $branch }? Azok a fájlok, amelyekben a két ág eltér, megváltoznak a lemezen.
git-done = git: { $said }
git-branches-title = Ágak: { $count }
git-log-title = Napló: { $count }
git-log-of-title = { $of } naplója: { $count }
btn-switch = Átvált
key-git = Git
key-git-diff = Diff
key-git-stage = Előkész
key-git-unstage = Kivesz
key-git-discard = Elvet
key-git-commit = Commit
key-git-show = Mutat
key-git-log = Napló
key-git-changes = Változ
key-git-pull = Pull
key-git-push = Push
key-git-branches = Ágak

undo-title = Visszavonás
btn-undo = Visszavon
menu-undo = Az utolsó művelet visszavonása
menu-strip-meta = Képek metaadatainak eltávolítása…
undo-nothing = nincs mit visszavonni: a program indulása óta nem volt másolás, mozgatás, Lomtárba törlés vagy új mappa
undo-copy =
    { $count ->
        [one] A másolással készült fájl a Lomtárba kerül:
       *[other] A másolással készült { $count } fájl a Lomtárba kerül:
    }
undo-move =
    { $count ->
        [one] A fájl visszakerül oda, ahol volt:
       *[other] A(z) { $count } fájl visszakerül oda, ahol volt:
    }
undo-trash =
    { $count ->
        [one] A fájl visszaáll a Lomtárból:
       *[other] A(z) { $count } fájl visszaáll a Lomtárból:
    }
undo-mkdir = Az új mappa törlése, ha még üres:
strip-title = { $count ->
        [one] Egy kép metaadatai
       *[other] { $count } kép metaadatai
    }
strip-all = Minden — szöveg, EXIF, XMP, IPTC, megjegyzések, C2PA
strip-ai = Csak amit egy képgeneráló modell írt bele — prompt, munkafolyamat, beállítások
strip-camera = Minden, kivéve a kamera EXIF-adatait és a szerzői jogot — GPS nélkül
strip-hint = Enter: eltávolítja; a kép nem lesz újrakódolva, a Ctrl+Alt+Z visszateszi · Esc
strip-nothing = nincs ezen a lemezen kijelölt vagy kurzor alatti PNG, WebP vagy JPEG
strip-done = { $count ->
        [one] egy kép
       *[other] { $count } kép
    } metaadatok nélkül ({ $blocks } blokk kivéve), { $clean } képben nem volt mit kivenni · a Ctrl+Alt+Z visszateszi
undo-restore = { $count ->
        [one] A kép visszaáll a metaadataival együtt:
       *[other] A(z) { $count } kép visszaáll a metaadataival együtt:
    }
undo-done =
    { $count ->
        [one] egy név visszakerült a helyére
       *[other] { $count } név visszakerült a helyére
    }
slow-drives = a meghajtólista { $secs } mp-ig tartott
slow-drives-late = a meghajtólista { $secs } mp-et várt: { $drives } nem válaszolt időben, ezért részletek nélkül látszik — hálózati meghajtó, amelynek nincs meg a kiszolgálója?
slow-listing = { $what } olvasása { $secs } mp-ig tartott
# editor search and undo (0.41)
btn-find-all = Minden találat
btn-replace-all = Mindet cseréli
btn-replace-one = Következőt cseréli
editor-replace-title = Csere
editor-replace-style = Stílus megőrzése (foo → bar, Foo → Bar, FOO → BAR)
editor-replace-groups = Reguláris kifejezésben $1 és { "${name}" } az, amit egy csoport elkapott, $$ pedig a dollárjel
editor-matches-title = Sorok ezzel: `{ $query }`: { $count }
editor-matches-hint = Gépeléssel szűkít · Enter: ugrás a sorra · Esc: vissza


# sorting and extended attributes (0.41)
opt-sort-case = Kis- és nagybetűk a rendezésben
opt-sort-case-auto = ahogy ez a rendszer teszi
opt-sort-case-yes = megkülönböztetve
opt-sort-case-no = egy ábécé
copy-keep-xattrs = Kiterjesztett attribútumok megőrzése

# editor conversions, disk diff and guides (0.41)
editor-convert-title = A fájl írása így
editor-convert-hint = Gépeléssel szűkít · Enter: átalakít (F2 kiírja, Ctrl+Z visszavonja) · Esc: vissza
editor-convert-eol = Sorvég: { $eol }
editor-convert-bom-on = Bájtsorrendjel (BOM): kiírva
editor-convert-bom-off = Bájtsorrendjel (BOM): nincs kiírva
editor-convert-page = Kódlap: { $page }
editor-converted-eol = { $name }: a következő mentéstől a sorok vége { $what }
editor-converted-bom-on = { $name }: a következő mentés bájtsorrendjelet ír
editor-converted-bom-off = { $name }: a következő mentés nem ír bájtsorrendjelet
editor-converted-page = { $name }: a következő mentés így ír: { $what }
editor-convert-same = A fájl már így van írva
editor-convert-no-bom = a(z) { $page } kódlapnak nincs bájtsorrendjele: csak az UTF-8-nak és az UTF-16-nak van
editor-convert-lossy = a(z) { $page } nem tudja tárolni { $name } minden karakterét: semmi sem változott
btn-reload = Újratölt
btn-what-changed = Mi változott
editor-diff-buffer = a szerkesztőben
editor-diff-hint = Ez az összehasonlítás csak megmutatja, mi változott — Esc visszavisz a kérdéshez


# job journal and saved syncs (0.41)
journal-left = Az előző munkamenetből befejezetlenül maradt másolások: { $count } · Alt+Shift+F6 listázza
journal-title = Befejezetlen feladatok
journal-hint = Enter folytat · Del eldob · Esc bezár
journal-none = Nem maradt befejezetlen feladat
journal-row = { $kind }: { $what } → { $target } · kész fájlok: { $done }
journal-row-unreadable = nem olvasható: { $file } ({ $error })
journal-unreadable = { $file }: nem olyan feladatnapló, amelyet ez a verzió olvasni tud ({ $error }). Az Alt+Shift+F6 listázza, ott a Del törli
journal-resumed = Folytatás: { $kind }: { $done } fájl már kész volt
journal-discarded = A befejezetlen feladat eldobva, azzal együtt, amit félig kiírt
journal-cannot-resume = Ez a napló nem olvasható; a Del törli
menu-jobs-left = Befejezetlen feladatok
sync-report = Jelentés írása a futásról ide (fájl)
sync-name = A pár mentése ezen a néven
sync-saved-button = Mentettek…
sync-saved-title = Mentett szinkronizálások
sync-saved-hint = Enter futtat · F4 módosít · Del elfelejt · Esc bezár
sync-saved-none = Még nincs mentett szinkronizálás: adjon nevet egynek a párbeszédablakban (Ctrl+Shift+Y)
sync-saved-removed = { $name } már nincs mentve
sync-report-local = A jelentésnek ezen a gépen lévő fájlnak kell lennie
sync-report-written = A szinkronizálás jelentése itt van: { $path }
sync-report-not-written = A jelentés nem volt kiírható ide: { $path }: { $error }
sync-report-title = Szinkronizálási jelentés
sync-report-name = Név: { $name }
sync-report-left = Bal: { $path }
sync-report-right = Jobb: { $path }
sync-report-to-right = Másolva balról → jobbra: { $count }
sync-report-to-left = Másolva jobbról → balra: { $count }
sync-report-cut-right = Törölve jobbról: { $count }
sync-report-cut-left = Törölve balról: { $count }
sync-report-skipped = Érintetlenül hagyva, bár eltérnek: { $count }
sync-report-failed = Sikertelen: { $count }
sync-report-stopped = Félúton leállt, így ezek vagy megtörténtek, vagy nem: { $count }
sync-report-not-done = Nem történt meg, mert egy korábbi lépés hibázott vagy leállt: { $count }
menu-sync-saved = Mentett szinkronizálások

# hex editor search
hex-search-wrapped = `{ $query }`: a fájl másik végéről újrakezdve megtalálva
hex-replace-title = Csere
hex-replace-with = Csere erre
btn-replace = Csere
hex-replace-ask = Lecseréli ezt: { $was }, itt: { $at }, erre: { $with }?
hex-replace-length = a csere { $with } bájt hosszú, a találat pedig { $was }: ez mindent eltolna utána — előbb nyomja meg az Ins billentyűt a beszúró módhoz

# pixels and video (0.41)
opt-viewer-pixels = Képek
opt-viewer-pixels-auto = valódi képpontok, ahol a terminál kirajzolja őket
opt-viewer-pixels-kitty = kitty grafikus protokoll
opt-viewer-pixels-iterm = iTerm2 beágyazott képek
opt-viewer-pixels-sixel = sixel
opt-viewer-pixels-off = mindig félblokkok
video-needs-program = ez videó ({ $what }): a képkockáit a(z) `{ $program }` rajzolja ki, amely nincs telepítve ezen a gépen — a Shift+F11 így is olvassa a konténert
video-no-frame = nincs képkocka ebből a videóból: { $why }
video-playing = lejátszás { $fps } képkocka/másodperccel, hang nélkül — Space szünet
video-paused = szünet itt: { $at } — Space folytatja, PgUp és PgDn tizedenként lép
video-ended = a videó véget ért — Space újra lejátssza


# finding things
menu-saved-searches = Mentett keresések
drive-searches = Mentett keresések
find-keys = F2 névvel megőrzi ezt a keresést · F9 a megőrzöttek
find-save-name = A keresés megőrzése ezen a néven
find-saved = a keresés megőrizve: { $name }
find-saved-none = még nincs megőrzött keresés: a keresőablakban az F2 megőriz egyet
find-saved-title = Mentett keresések — Enter megnyit, F4 szerkeszt, Del elfelejt
find-saved-running = keresés: { $name }…
find-saved-gone = a(z) { $name } keresés elfelejtve
find-loaded = a(z) { $name } keresés betöltve
find-fuzzy = Hozzávetőleges név: a fenti betűk, sorrendben, a legjobb találat elöl
find-meta = Metaadatok (artist=bowie; camera=)
find-index = A rendszer indexének megkérdezése bejárás helyett (új fájlok kimaradhatnak)
tools-for-index = az index, amelyet az Alt+F7 bejárás helyett megkérdezhet
menu-frecent = Gyakori mappák
frecent-none = még egyetlen mappát sem látogatott meg
frecent-gone = { $path } kikerült a gyakori mappák közül
frecent-hint = gépelje az útvonal betűit · Enter odalép · Del elfelejt · Esc bezár

# ai third round
ai-redact-off = A kitakarás ki van kapcsolva: minden úgy megy, ahogy van
ai-redact-none = Nincs kitakarandó titok
ai-redact-count = Küldés előtt kitakart titkok: { $count } ({ $kinds })
ai-redacted = kitakart titkok: { $count }
ai-preview-title = Mi megy ide: { $name } ({ $host })
ai-preview-instruction = Utasítás
ai-preview-earlier = Az eddigi beszélgetés
ai-preview-question = A kérdés és a fájljai
ai-preview-key = F3 megmutatja, mi megy el
ai-preview-reading = Az elküldendők beolvasása…
key-ai-redact = Kitakar
ai-redact-now-on = A kitakarás be van kapcsolva: a kulcsok, tokenek, privát kulcsok és jelszavak küldés előtt ki lesznek takarva
ai-redact-now-off = A kitakarás ki van kapcsolva: ami elmegy, úgy megy, ahogy van
ai-panel-redact-on = titkok kitakarva
ai-panel-redact-off = titkok nincsenek kitakarva
key-ai-local = Helyi
ai-row-refused = nem ez a gép: elutasítva
ai-panel-local-only = csak helyi
ai-local-now-on = A „csak helyi” be van kapcsolva: a nem ezen a gépen lévő szolgáltatás elutasítva, semmi sem megy el hozzá
ai-local-now-off = A „csak helyi” ki van kapcsolva: a lista bármely szolgáltatása kérdezhető
ai-local-refused = A „csak helyi” be van kapcsolva: { $name } itt válaszol: { $host }, ami nem ez a gép — semmi sem ment el (az AI panelen az F6 kikapcsolja)
ai-rename-prompt = Mi legyen a(z) { $count } névből? (szavakkal)
ai-send-names = Az utasítás és { $count } név a(z) { $total } közül — csak a nevek, a fájlokból semmi — ide megy: { $host }.
ai-rename-ready = a modell mintája a párbeszédablakban van: nézze meg az előnézetet, javítsa, ami hibás, és csak az OK nevez át
ai-rename-waiting = a modell átnevezési mintája kész: a Ctrl+Shift+F6 megnyitja vele a párbeszédablakot
ai-rename-unreadable = a modell nem átnevezési mintával válaszolt: a válasza a válaszok mappájában van (ai:), és semmi sem lett átnevezve
ai-rewrite-prompt = Mi legyen a kijelölt blokkból? (szavakkal)
ai-rewrite-no-block = előbb jelöljön ki egy blokkot (Shift+nyilak): az átírás pontosan a kijelöltet cseréli, az oszlopblokknak pedig nincs alakja, amelybe vissza lehetne tenni
ai-rewrite-read-only = ez a fájl itt csak olvasható: az átírásnak nem lenne hová kerülnie
ai-rewrite-before = a blokk
ai-rewrite-after = ahogy a modell átírta
ai-rewrite-hint = A modell átírása a blokkal szemben — Enter a blokk helyére teszi, Esc úgy hagyja a blokkot
diff-hint-rewrite = Enter cseréli a blokkot · Esc úgy hagyja
ai-rewrite-applied = a blokk helyére a modell átírása került — egy Ctrl+Z visszateszi
ai-rewrite-left = a blokk úgy maradt, ahogy volt
ai-rewrite-stale = a blokk már nem az, ami az átírás kérésekor volt: semmi sem lett cserélve — a modell változata a válaszok mappájában van (ai:)
ai-rewrite-same = a modell változatlanul adta vissza a blokkot: nincs mit cserélni
ai-answer-kept = a válasz { $name } néven a válaszok mappájában van (ai:) — a szerkesztő elöl maradt
ai-failure-question = Ez a parancs { $code } kilépési kóddal ért véget: `{ $command }` — miért hibázott, és mi javítaná meg? Röviden.
ai-send-failure = A hibás parancs és kimenetének utolsó { $lines } sora ide megy: { $host }.

# archives and jobs
arc-test-whole = { $name }: { $files } bejegyzés tesztelve, { $bytes } — mind ép
arc-test-bad-title = { $name }: { $bad } bejegyzés a(z) { $files } közül nem jött ki épen

# extending it
plugin-column-failed = { $column }: { $error } — az oszlopa üres marad azoknál a fájloknál, amelyekre nem válaszolt
plugins-card-columns = Oszlopok (ahogy egy [[column_set]] nevezi őket):
usermenu-far-row = Amit másképp olvastunk a FarMenu.ini fájlból ({ $count })
usermenu-far-notes = a menü a FAR FarMenu.ini fájljából jön: { $count } dolgot másképp olvastunk — az utolsó sor felsorolja őket
usermenu-far-not-run = Semmi sem futott le.
find-age = Módosítva, ennyi napja (7, >=30, 7-30)
find-bad-age = `{ $text }` nem napokban megadott kor — { $error }
find-label-age = { $days } n
find-keys-ai = A Ctrl+Alt+Q szavakkal mondja el a keresést: egy nyelvi modell tölti ki ezeket a sorokat
ai-find-prompt = Mit keres? (szavakkal)
ai-send-words = Ezek a szavak — és a panelekből semmi: se név, se útvonal, se fájl — ide mennek: { $host }.
ai-find-ready = a modell keresése a párbeszédablakban van: olvassa el a sorokat, javítsa, ami hibás, és csak az OK keres
ai-find-waiting = a modell keresése kész: az Alt+F7 megnyitja vele a párbeszédablakot
ai-find-unreadable = a modell nem kereséssel válaszolt: a válasza a válaszok mappájában van (ai:), és semmit sem kerestünk
ai-find-left-out = { $said } — NINCS a sorokban, mert azok nem tudják kifejezni: { $what }
# treemap and compare
usage-map-title = Lemezhasználat, a teljes fa: { $path }
usage-map-walking = Mérés… fájlok: { $files }, eddig { $size }
usage-map-where = { $path } · { $size } · fájlok: { $files }
usage-map-unreadable = be nem olvasott mappák: { $count }
usage-map-rest = kisebb fájlok: { $count }
usage-map-others = egyebek: { $count }
usage-map-empty = Ebben a mappában semmi sem foglal helyet
usage-map-chosen = { $name } · { $size } · { $share }%
usage-map-hint = ←↑↓→ · Enter belép · BkSp fel · Tab lista · G panel oda · Esc
usage-map-hint-list = ↑↓ · Enter belép · BkSp fel · Tab térkép · G panel oda · Esc
usage-map-hint-walking = Esc leállít
usage-map-top = Ez a megmért mappa: itt nincs fölötte semmi
usage-map-no-file = Ez a sor sok kis fájl, nem egyetlen, amelyre a panel ráállhat
bin-comparing = A két fájl összehasonlítása bájtonként… Esc leállítja
bin-bytes = { $count } bájt
bin-same = A két fájl bájtra azonos: mindkettő { $bytes } bájt
bin-where = { $at }. különbség / { $count } · az első eltolása: { $offset } · eltérő bájtok: { $differing }
bin-hint = Alt+↑↓ különbségek · ↑↓ PgUp PgDn Home End sorok · Esc bezár
diff-unified-on = Egy oszlop: ami eltűnt, aztán ami jött
diff-unified-off = Két oszlop, egymás mellett
diff-patch-title = A különbségek mentése patchként (.diff)
diff-patch-saved = { $name } megírva: { $lines } soros egyesített diff
diff-patch-nothing = A két fájl azonos: nincs menthető patch
# wastebasket and external editor
drive-trash = Lomtár
drive-trash-closed = Lomtár (ez a rendszer nem engedi, hogy egy program kilistázza)
menu-trash = Lomtár
menu-trash-empty = Lomtár ürítése
key-trash-restore = Vissza
key-trash-empty = Ürít
btn-empty = Ürít
btn-rename = Átnevez
trash-closed = ennek a rendszernek a Lomtára csak a saját fájlkezelőjéből nyitható meg
trash-empty = a Lomtár üres
trash-failed = a Lomtár: { $why }
trash-restored =
    { $count ->
        [one] egy elem visszakerült a helyére
       *[other] { $count } elem visszakerült a helyére
    }
trash-purged =
    { $count ->
        [one] egy elem véglegesen törölve
       *[other] { $count } elem véglegesen törölve
    }
trash-card-name = Név
trash-card-from = Itt volt
trash-card-deleted = Törölve
trash-card-size = Méret
trash-clash-title = Visszaállítás
trash-clash-body = { $path } már létezik.
trash-clash-note = A Felülír a meglévőt a Lomtárba küldi; az Átnevez ezt mellé teszi vissza.
trash-clash-more =
    { $count ->
        [one] Még egy elem van ugyanebben a helyzetben.
       *[other] Még { $count } elem van ugyanebben a helyzetben.
    }
trash-purge-title = Végleges törlés
trash-purge-body =
    { $count ->
        [one] Törli a Lomtárból: { $name }?
       *[other] Töröl { $count } elemet a Lomtárból?
    }
trash-purge-note = Ez végleges törlés: semmi sem hozza vissza őket.
trash-empty-title = Lomtár ürítése
trash-empty-body = Töröl mindent a Lomtárból?
opt-editor-external = Külső szerkesztő
opt-editor-use-external = Külső szerkesztő az F4-re (az Alt+F4 ekkor a beépített)
opt-viewer-external = Külső nézőke
opt-viewer-use-external = Külső nézőke az F3-ra (az Alt+F3 ekkor a beépített)


# editor features
opt-editor-line-numbers = Sorok számozása
editor-line-numbers-on = Sorszámok bekapcsolva
editor-line-numbers-off = Sorszámok kikapcsolva
editor-prompt-pipe-block = A blokk átengedése ezen a parancson:
editor-prompt-pipe-file = Az egész fájl átengedése ezen a parancson:
editor-prompt-insert-output = Beszúrja, amit ez a parancs kiír:
editor-prompt-insert-file = Fájl beszúrása:
editor-prompt-write-block = A blokk kiírása fájlba:
editor-pipe-running = Fut: { $command } …
editor-pipe-done = { $command }: lecserélve arra, amit kiírt, sorok: { $lines } (Ctrl+Z visszavonja)
editor-pipe-inserted = { $command }: beszúrt sorok: { $lines }
editor-pipe-empty = { $command } semmit sem írt ki: semmi sem lett beszúrva
editor-pipe-failed = { $command }: { $error } — semmi sem változott
editor-pipe-changed = { $command }: a szöveg megváltozott, amíg a parancs futott — semmi sem lett cserélve
editor-pipe-gone = { $command }: a fájl, amelyre futott, már nincs nyitva — semmi sem változott
editor-pipe-column = Oszlopblokk nem engedhető át parancson: jelöljön ki egész sorokat, vagy semmit az egész fájlhoz
editor-file-inserted = { $name } beszúrva, sorok: { $lines }
editor-insert-not-local = A szerkesztőbe csak ezen a gépen lévő fájl olvasható be, és csak ilyenbe írható ki
editor-insert-too-big = túl nagy a szerkesztőbe való beolvasáshoz
editor-block-written = A blokk kiírva ide: { $name }: { $size }
editor-block-write-self = { $name } az éppen szerkesztett fájl: a blokk nem lett ráírva
editor-choice-hint = Gépeléssel szűkít · Enter: választ · Esc: vissza
editor-syntax-title = A fájl színezése így
editor-syntax-auto = Amit a neve mond: { $language }
editor-syntax-auto-none = Amit a neve mond: semmi ismert
editor-syntax-none = Sima szöveg, színek nélkül
editor-syntax-set = { $name } mostantól így olvasandó: { $language }
editor-syntax-off = { $name } mostantól sima szövegként olvasandó
editor-case-heading = Kis- és nagybetű váltása
editor-case-upper = NAGYBETŰS
editor-case-lower = kisbetűs
editor-case-title = Szókezdő Nagybetűk: minden szó elején nagybetű
editor-case-toggle = fORDÍTOTT: a nagyból kicsi, a kicsiből nagy lesz
editor-no-word = Nincs kijelölve semmi, és a kurzor nem szón áll
editor-prompt-isearch = Keresés gépelés közben (↓ ↑ következő és előző, Esc visszalép)
editor-word-only = A kurzor alatti szó máshol nem fordul elő
editor-not-on-word = A kurzor nem szón áll
editor-prompt-reflow = Hány oszlopra tördelje a bekezdést? (csak Enter: { $width })
editor-reflowed = A bekezdés { $width } oszlopra tördelve (Ctrl+Z visszavonja)
editor-reflow-bad-width = { $width } nem szélesség: 8 és 1000 közötti oszlopszám kell
editor-no-paragraph = A kurzor üres soron áll: itt nincs bekezdés
editor-no-name-yet = Ennek a fájlnak még nincs neve: előbb mentse el
editor-tools-title = Szerkesztő parancsok
editor-tool-isearch = Keresés gépelés közben
editor-tool-word-next = A kurzor alatti szó következő előfordulása
editor-tool-word-prev = A kurzor alatti szó előző előfordulása
editor-tool-pipe = A blokk vagy az egész fájl átengedése egy parancson
editor-tool-write-block = A blokk kiírása fájlba
editor-tool-case = A blokk kis- és nagybetűinek váltása
editor-tool-reflow = A bekezdés újratördelése adott szélességre
editor-tool-insert-file = Fájl beszúrása a kurzornál
editor-tool-insert-output = Egy parancs kimenetének beszúrása
editor-tool-insert-date = Dátum és idő beszúrása
editor-tool-insert-name = A fájl nevének beszúrása
editor-tool-insert-path = A fájl teljes útvonalának beszúrása
editor-tool-line-numbers = Sorszámok be vagy ki
editor-tool-syntax = A fájl színezési nyelvének kiválasztása

# Key bar labels: short, they must fit in a few columns.
key-help = Pomoc
key-menu = MenuUż
key-view = Podgląd
key-edit = Edytuj
key-copy = Kopiuj
key-print = Drukuj
key-move = ZmNazw
key-mkdir = UtwFol
key-delete = Usuń
key-pulldown = Konfig
key-quit = Koniec
# The key bar while a modifier is held: FAR's own words, six columns wide.
key-add = Dodaj
key-extract = Rozpak
key-arccmd = Polec
key-edit-new = Edytuj…
key-rename = ZmNazw
key-group = Grupa
key-selup = ZaznGór
key-left = Lewy
key-right = Prawy
key-sort-name = Nazwa
key-sort-ext = Rozsz
key-sort-write = Zapis
key-sort-size = Rozm
key-sort-unsort = BezSor
key-sort-create = Utworz
key-sort-access = Użycie
key-sort = Sortuj
key-mklink = Dowiąż
key-find = Znajdź
key-history = Histor
key-folders = HisFol
key-plugins = Wtyczki

# Panel chrome
list-up = W górę
list-folder = Folder
col-name = Nazwa
col-ext = Rozsz
col-size = Rozmiar
col-allocated = Przydzielono
col-modified = Zmieniono
col-created = Utworzono
col-accessed = Użycie
col-attr = Atr
col-owner = Właściciel
col-group = Grupa
col-descr = Opis
col-links = Dow
col-git = Git
descr-title = Opis { $name }
descr-set = opisano { $name }
descr-cleared = usunięto opis { $name }
descr-not-here = opisy są przechowywane obok folderu na tym komputerze
col-date-time = Data       Czas
panel-list-count = elementów na liście: { $count }
panel-empty = (pusto)
panel-loading = odczyt…

# Prompts
prompt-select-mask = Zaznacz pliki według maski:
prompt-deselect-mask = Odznacz pliki według maski:
prompt-goto = Przejdź do folderu:
prompt-filter = Maska plików lub część nazwy:
prompt-search = Szybkie szukanie:

# Dialogs
dialog-error = Błąd
dialog-help-hint = Esc wraca do spisu treści · F7 szuka we wszystkich rozdziałach · F1 zamyka · ↑↓ PgUp PgDn przewijają
dialog-help-index = FinalCommander { $version } — Pomoc
dialog-help-hint-index = Enter otwiera rozdział · F7 szuka we wszystkich rozdziałach · Esc zamyka
help-search-prompt = Szukaj w pomocy
help-search-none = Żaden wiersz pomocy nie zawiera „{ $query }”
help-search-title = „{ $query }” w pomocy: wierszy — { $count }
output-search-prompt = Znajdź w wyjściu
output-search-empty = Nic jeszcze nie zostało wypisane
output-search-found = Wiersz { $line } z { $lines }
output-search-wrapped = Wiersz { $line } z { $lines }, ponownie od najnowszego
output-search-none = Żaden wiersz wyjścia nie zawiera „{ $query }”

# Status messages
msg-not-implemented = { $command } nie jest jeszcze zaimplementowane w tej kompilacji
msg-selected = zaznaczono elementów: { $count }
msg-deselected = odznaczono elementów: { $count }
msg-mask-invalid = nieprawidłowa maska: { $error }
msg-theme = motyw: { $name }
msg-font-bigger = czcionka: terminal poproszono o większą
msg-font-smaller = czcionka: terminal poproszono o mniejszą
msg-font-reset = czcionka: terminal poproszono o początkową
msg-sort = sortowanie: { $key }{ $reverse }
msg-sort-groups-on = grupy sortowania włączone ({ $count })
msg-sort-groups-off = grupy sortowania wyłączone ({ $count })
msg-sort-groups-none = nie zdefiniowano grup sortowania: zobacz [[sort_group]] w konfiguracji
msg-view = widok: { $mode }
msg-hidden-on = pliki ukryte są pokazywane
msg-hidden-off = pliki ukryte są schowane
msg-no-history = nie ma więcej historii
msg-tab-last = panel zawsze ma jedną kartę
msg-tab-end = karta jest już na tym końcu rzędu
tabs-title = Karty
msg-tab-moved = kartę przeniesiono do drugiego panelu
msg-tab-locked = ta karta jest przypięta do swojego folderu
msg-tab-locked-on = kartę przypięto do jej folderu
msg-tab-locked-off = kartę odpięto
msg-bookmark-set = skrót { $digit }: { $path }
msg-bookmark-unset = skrót { $digit } nie jest ustawiony
msg-bookmark-added = zapamiętano { $path }
msg-bookmark-removed = porzucono { $path }
msg-bookmark-known = ten folder jest już na liście
msg-dirsize = { $path }: { $bytes }, plików: { $files }
msg-dirsize-many = mierzenie folderów: { $count }…
usage-title = Użycie dysku: { $path } · { $size } · Tab: całe drzewo jako mapa
usage-not-here = użycie dysku mierzy się dla folderu na dysku, a nie w archiwum czy na liście znalezionych plików
msg-dirsize-no-folder = nie ma tu folderu do zmierzenia

# Jobs: prompts, confirmations, progress, conflicts
prompt-copy-to = Kopiuj { $what } do:
prompt-move-to = Zmień nazwę lub przenieś { $what } do:
prompt-mkdir = Utwórz folder:
prompt-tab-name = Nazwa tej karty:
prompt-bookmark-name = Nazwa tego folderu:
what-selected = zaznaczone elementy ({ $count })
confirm-delete-title = Usuń
confirm-delete-body =
    { $count ->
        [one] Usunąć { $name } ({ $bytes })?
        [few] Usunąć { $count } elementy ({ $bytes })?
       *[other] Usunąć { $count } elementów ({ $bytes })?
    }
prompt-link = Utwórz dowiązanie do { $name } jako:
link-created = utworzono { $name }
confirm-wipe-title = Wymaż
confirm-wipe-body =
    { $count ->
        [one] Nadpisać i usunąć { $name } ({ $bytes })?
        [few] Nadpisać i usunąć { $count } elementy ({ $bytes })?
       *[other] Nadpisać i usunąć { $count } elementów ({ $bytes })?
    }
confirm-wipe-honest = Bajty są nadpisywane jeden raz, po czym plik jest usuwany. Na dysku SSD, w systemie plików z kopiowaniem przy zapisie lub z migawkami starsze kopie mogą przetrwać — a nazwa zostaje w katalogu i w dzienniku.
job-copy = Kopiowanie
job-move = Przenoszenie
job-delete = Usuwanie
job-wipe = Wymazywanie
job-trash = Przenoszenie do kosza
job-replace = Zamienianie
job-pack = Pakowanie
job-rebuild = Zapisywanie archiwum

# ── the attributes dialog (SPEC 10 §3.8) ───────────────────────────────────────────────
attr-title = Atrybuty pliku
attr-count = zaznaczonych elementów: { $count }
attr-now = teraz { $text } ({ $octal })
attr-owner-read = Właściciel może czytać
attr-owner-write = Właściciel może zapisywać
attr-owner-exec = Właściciel może uruchamiać
attr-group-read = Grupa może czytać
attr-group-write = Grupa może zapisywać
attr-group-exec = Grupa może uruchamiać
attr-other-read = Wszyscy mogą czytać
attr-other-write = Wszyscy mogą zapisywać
attr-other-exec = Wszyscy mogą uruchamiać
attr-setuid = Uruchamiaj jako właściciel (setuid)
attr-setgid = Uruchamiaj jako grupa (setgid)
attr-sticky = Usuwać może tylko właściciel (sticky)
attr-readonly = Tylko do odczytu
attr-hidden = Ukryty
attr-system = Systemowy
attr-archive = Archiwalny
attr-temporary = Tymczasowy
attr-offline = Offline
attr-indexed = Indeksator może go przeglądać
attr-compressed = skompresowany
attr-encrypted = zaszyfrowany
attr-sparse = rzadki
attr-reparse = punkt analizy
attr-also = także { $what }
attr-mtime = Zmieniono
attr-atime = Ostatni odczyt
attr-btime = Utworzono
attr-ctime = Zmiana wpisu
attr-btime-readonly = (czas utworzenia można ustawić tylko w Windows)
attr-utc = Czasy w UTC
attr-owner = Właściciel (użytkownik:grupa)
attr-owner-is = właściciel { $who }
btn-attr-original = Pierwotne
btn-attr-current = Bieżące
btn-attr-blank = Wyczyść
attr-recursive = Oraz wszystko wewnątrz folderów
attr-bad-time = `{ $text }` nie jest datą i godziną (YYYY-MM-DD HH:MM:SS)
attr-bad-owner = żadne konto ani grupa nie nazywa się tu `{ $text }`
attr-in-archive = ta zmiana zapisuje archiwum od nowa, pozostałe wpisy zostają bez zmian
job-attrs = Ustawianie atrybutów

# ── the copy dialog (SPEC 03 §4) ───────────────────────────────────────────────────────
copy-title = Kopiuj
copy-title-move = Zmień nazwę lub przenieś
copy-target = Do
copy-conflict = Jeśli plik już istnieje
copy-conflict-ask = zapytaj
copy-conflict-overwrite = zastąp
copy-conflict-newer = zastąp, gdy źródło jest nowsze
copy-conflict-skip = pomiń
copy-conflict-rename = nadaj kopii wolną nazwę
copy-conflict-append = dołącz na końcu
copy-only-newer = Tylko pliki nowsze od istniejących
copy-filter = Tylko nazwy pasujące do maski
copy-follow-links = Kopiuj to, na co wskazuje dowiązanie, a nie dowiązanie
copy-keep-times = Zachowaj czasy
copy-keep-mode = Zachowaj uprawnienia
copy-verify = Odczytaj każdy plik ponownie i porównaj
copy-several = miejsc docelowych: { $count }, jedno po drugim

# ── the terminal (SPEC 06 §4) ──────────────────────────────────────────────────────────
term-started = terminal, uruchomiono { $shell }
term-hint = Ctrl+Alt+E: powrót do paneli
term-ended = zakończył się — dowolny klawisz to zamknie
job-mkdir = Tworzenie folderu
job-planning = planowanie…
job-counters = plików: { $files } / { $filestotal } · { $bytes } / { $bytestotal }
job-rates = { $speed }/s · upłynęło { $elapsed } · pozostało { $eta }
job-rates-no-eta = { $speed }/s · upłynęło { $elapsed }
job-hint = Esc anuluje
job-hint-queue = Esc anuluje · B w tle
job-queued = { $kind } czeka w kolejce, nieukończonych zadań przed nim: { $ahead } · Alt+Shift+F5 pokazuje listę
job-background = { $kind } trwa w tle · Alt+Shift+F5 pokazuje zadania
job-asks = { $kind } czeka na odpowiedź · Alt+Shift+F5
job-paused = { $kind }: wstrzymano
job-resumed = { $kind }: wznowiono
job-retry-nothing = Nie ma czego ponawiać
job-still-waiting = To zadanie czeka na swoją kolej
jobs-title = Zadania (nieukończonych: { $active })
jobs-hint = Enter pokaż · Space pauza · Del zatrzymaj · F5 ponów · Ctrl+↑↓ kolejność · Esc zamknij
jobs-quit-title = Zadania nadal trwają
jobs-quit-body = Te zadania zostaną przerwane, jeśli program zakończy się teraz:
job-state-running = trwa
job-state-paused = wstrzymane
job-state-asking = pyta
job-state-queued = czeka
job-state-held = zatrzymane
job-state-done = gotowe
job-state-failed = błędy
job-state-cancelled = anulowane
job-done = { $kind }: plików: { $files }, { $bytes }
job-done-renamed = { $kind }: plików przeniesionych bez kopiowania: { $files }
job-done-skipped = , pominięto: { $skipped }
job-done-verified = , zweryfikowano: { $verified }
job-done-unverified = , nie udało się tu odczytać ponownie: { $unverified }
job-done-errors = , błędów: { $errors }
job-cancelled = { $kind }: anulowano
job-errors-title = Błędy
conflict-title = Cel już istnieje
conflict-dst = cel     { $path }   { $bytes }
conflict-src = źródło  { $path }   { $bytes }
conflict-newer = źródło jest nowsze
conflict-older = cel jest nowszy
btn-overwrite = Zastąp
btn-overwrite-all = Wszystkie
btn-skip = Pomiń
btn-skip-all = Pomiń wszystkie
btn-retry = Ponów
btn-move = Przenieś
btn-delete = Usuń
btn-wipe = Wymaż
btn-copy = Kopiuj
btn-discard = Odrzuć
btn-kill = Zakończ
btn-print = Drukuj
btn-quit = Wyjdź
btn-play = Odtwórz
btn-run = Uruchom
btn-accept = Akceptuj
btn-cancel = Anuluj
msg-nothing-to-do = nic nie zaznaczono
cmd-complete-nothing = nie ma czym tego dokończyć
complete-several = nazw zaczynających się w ten sposób: { $count }
form-moving = strzałki przesuwają okno · Enter lub Esc zostawia je tam
form-moved = okno zostaje tam, gdzie je umieszczono
msg-trash-refused = Kosz tego nie przyjął — Shift+Del usuwa od razu.

# Checksums (SPEC 03 §9)
sum-title = Sumy kontrolne
sum-algorithm = Hash
sum-one-file = Jedna lista dla wszystkich
sum-name = Nazwa
sum-working = { $name }: { $done } z { $total }
sum-verifying = sprawdzanie według { $name }
sum-stopped = zatrzymano
sum-written = zapisano { $name }
sum-no-name = lista potrzebuje nazwy
sum-local-only = sumę kontrolną można obliczyć tylko dla pliku na tym komputerze
print-title = Drukowanie
print-body = Wydrukować {$what}?
print-sending = wysyłanie do drukarki…
print-sent = wysłano do drukarki
print-failed = drukowanie nie powiodło się ({$line}): {$error}
print-local-only = wydrukować można tylko plik na tym komputerze
print-no-command = brak polecenia drukowania: ustaw [print] command lub zainstaluj lp albo lpr
sum-all-match = wszystkie { $count } się zgadzają
sum-verify-title = Sumy kontrolne
sum-verify-summary = zgodnych: { $ok }, niezgodnych: { $bad }
sum-differs = nie zgadza się
sum-missing = brak
sum-unreadable = nie da się odczytać

# Synchronising two directories (SPEC 03 §10)
sync-title = Synchronizacja
sync-between = { $left }  ↔  { $right }
sync-recursive = Także wszystko w podfolderach
sync-both-ways = W obie strony, wygrywa nowszy
sync-delete-extra = Proponuj usunięcie tego, czego nie ma po drugiej stronie
sync-by-content = Porównuj bajty, a nie rozmiar i czas
sync-filter = Tylko nazwy pasujące do maski
sync-walking = przeglądanie { $at } — jak dotąd { $pairs }
sync-planned = do skopiowania: { $copies } ({ $bytes }), do usunięcia: { $deletes }
sync-hint = Space następna odpowiedź · ← → kierunek · = zostaw · F2 wykonaj · Esc anuluj
sync-hint-walking = Esc zatrzymuje przeglądanie
sync-nothing-to-do = każdy wiersz mówi, żeby zostawić wszystko tak, jak jest
sync-local-only = synchronizacja może czytać i zapisywać tylko na tym komputerze
sync-same-directory = oba panele są w tym samym folderze

# What to run for a kind of file (SPEC 06 §5)
assoc-none = nic tu nie mówi, jak otworzyć { $name }
usermenu-empty = Nie ma jeszcze menu użytkownika: utwórz je w { $path }
usermenu-last-gone = wiersza wybranego ostatnio nie ma już w tym menu
usermenu-bad-condition = Niezrozumiały warunek — { $problem }
assoc-desktop = Przekaż pulpitowi
assoc-open-with = Otwórz { $name } za pomocą
assoc-no-pager = ani przypisanie, ani $PAGER nie mówi, czym to czytać
assoc-no-editor = ani przypisanie, ani $VISUAL nie mówi, czym to edytować

# Viewer (F3)
key-wrap = Zawiń
key-hex = Hex
key-text = Tekst
key-dump = Zrzut
key-doc = Dok
key-search = Szukaj
key-code = StrKod
key-follow = Śledź
viewer-prompt-search = Szukaj ({ $mode }):
viewer-prompt-goto = Idź do przesunięcia lub NN% :
viewer-not-found = nie znaleziono `{ $query }`
viewer-follow-on = śledzenie końca pliku
viewer-follow-off = tryb śledzenia wyłączony
viewer-codepage = strona kodowa: { $name }
viewer-wrap-mode = { $mode }
viewer-no-undo = nie ma wcześniejszej pozycji
viewer-empty = (pusty plik)
viewer-not-markdown = to nie jest plik Markdown albo jest za duży, by go złożyć
viewer-not-table = to nie jest plik tabelaryczny albo jest za duży, by odczytać go w całości
viewer-base64-done = Zdekodowano: { $bytes }
viewer-not-base64 = Ten plik nie jest base64: { $why }
viewer-status = { $mode } { $codepage }  { $offset } / { $len }  { $percent }%

key-disasm = Dezasm
viewer-disasm-failed = nie można zdezasemblować { $name }: { $error }
viewer-decompile-running = { $tool } pracuje nad { $name }…
viewer-decompile-none = nic tu nie potrafi zdekompilować { $format }; zainstaluj narzędzie i dodaj regułę [[decompile]] do fcmd.toml
arc-needs-program = to archiwum czyta `{ $program }`, który nie jest zainstalowany na tym komputerze
viewer-decompile-failed = { $tool }: { $error }
viewer-blame-derived = git blame dotyczy samego pliku: najpierw do niego wróć
viewer-derived = { $producer } dla { $name }

# Editor (F4)
key-save = Zapisz
key-replace = Zamień
editor-prompt-goto = Idź do wiersza lub wiersz:kolumna :
editor-prompt-save-as = Zapisz jako:
editor-saved = { $name }: zapisano, bajtów: { $bytes }
editor-nothing-to-save = { $name } nie ma niezapisanych zmian
editor-not-found = nie znaleziono `{ $query }`
editor-replaced = zamian: { $count }
editor-read-only = { $name } jest tylko do odczytu
editor-readonly-on = tylko do odczytu: zmiany są odrzucane
editor-readonly-off = tryb tylko do odczytu wyłączony
editor-no-undo = nie ma już czego cofać
editor-no-redo = nie ma czego ponawiać
editor-discard-title = Niezapisane zmiany
editor-discard-body = { $name } ma niezapisane zmiany. Wyjść i je utracić?
editor-changed-title = Zmieniono na dysku
editor-changed-body = { $name } zmienił się na dysku od chwili otwarcia. Zapisać mimo to?
editor-reloaded = { $name } wczytano ponownie z dysku
editor-failed = { $name }: { $error }

# netbox (SPEC 04 §6)
net-sites-title = Połączenia
net-sites-hint = Enter połącz · n URL · Ins zapisz · F4 edytuj · d rozłącz · Esc zamknij
net-no-sites = Nie ma jeszcze witryn. Naciśnij `n`, aby wpisać URL, albo dodaj bloki [[site]] do konfiguracji.
net-not-a-stored-session = ten wiersz to otwarte połączenie, a nie zapisana sesja
net-site-forgotten = { $name } nie jest już zapisane
net-prompt-url = Połącz z:
net-prompt-password = Hasło dla { $user }@{ $host }:
net-prompt-passphrase = Hasło klucza { $user }@{ $host }:
net-connecting = łączenie z { $host }…
net-connected = połączono z { $host }
net-disconnected = rozłączono { $host }
net-not-connected = { $host } nie jest połączony
net-local-panel = ten panel jest w lokalnym systemie plików
net-nothing-pending = żadne połączenie na to nie czeka
net-hostkey-title = Nieznany klucz hosta
net-hostkey-body = { $host } nie ma w known_hosts. Jego odcisk to:
net-hostkey-question = Zapisać go i połączyć?
net-hostkey-changed-title = Klucz hosta ZMIENIŁ SIĘ
net-hostkey-changed-body = { $host } odpowiedział kluczem innym niż zapisany.
net-hostkey-changed-where = Zapisany jest w { $path }, wiersz { $line }.
net-hostkey-changed-what = Odtworzony serwer wygląda dokładnie tak samo jak ktoś pośrodku. Nic nie jest tu akceptowane: otwórz plik, przeczytaj wiersz i usuń go samodzielnie, jeśli wiesz, dlaczego klucz się zmienił.
btn-open-file = Otwórz plik
net-fetching = pobieranie { $name }…
net-viewing-copy = { $name }: kopia lokalna, pobrana do podglądu
net-too-big = { $name } jest większy niż { $limit }: zwiększ net.fetch_limit_mb, aby go pobrać
net-editing = { $name }: edycja kopii lokalnej; zapis odsyła ją z powrotem
net-writing-back = odsyłanie { $name }…
net-written-back = { $name }: odesłano, bajtów: { $bytes }
net-write-back-failed = odesłanie nie powiodło się: { $error } — twoje zmiany są w { $local }
net-changed-title = Zmieniono na serwerze
net-changed-body = { $name } zmienił się na serwerze od chwili pobrania. Nadpisać go?
confirm-delete-permanent = To usuwa trwale: dla tych elementów nie ma kosza.
confirm-trash-title = Przenieś do kosza
confirm-trash-body =
    { $count ->
        [one] Przenieść { $name } ({ $bytes }) do kosza?
        [few] Przenieść { $count } elementy ({ $bytes }) do kosza?
       *[other] Przenieść { $count } elementów ({ $bytes }) do kosza?
    }
confirm-trash-where = Trafiają tam, gdzie pulpit trzyma usunięte pliki, i można je stamtąd przywrócić.

# ---------------------------------------------------------------- menu bar (SPEC 10)
menu-left = &Lewy
menu-right = &Prawy
menu-files = Pl&iki
menu-commands = Pol&ecenia
menu-options = &Opcje
menu-brief = &Skrótowy
menu-medium = Ś&redni
menu-full = &Pełny
menu-detailed = Ze sz&czegółami
menu-owners = &Właściciele plików
menu-links = Dowiąza&nia plików
menu-alt-full = Pełny, rozmi&ary w całości
menu-descriptions = &Opisy
menu-long-descriptions = &Długie opisy
menu-wide = S&zeroki
menu-info-panel = Panel informacy&jny
menu-quick-view = Sz&ybki podgląd
menu-tree-panel = Drz&ewo
menu-tab-new = Nowa &karta
menu-tab-close = Zamknij kartę
menu-tab-next = Nast&ępna karta
menu-tab-prev = Poprzednia karta
menu-tab-move = Przenieś kartę do drugiego panel&u
menu-tab-lock = Przypnij kartę do jej &folderu
menu-tab-name = Nazwij tę kartę
menu-tab-list = Wszystkie karty
menu-sort-modes = Try&by sortowania
menu-hidden = Pliki ukryte i systemowe
menu-filter = Fil&tr plików
menu-filter-sets = Nazwane filtry
menu-panel-onoff = Włącz/wyłącz pane&l
menu-reread = Odśw&ież
menu-change-drive = Z&mień napęd
menu-view = &Podgląd
menu-edit = &Edytuj
menu-edit-new = Edytuj nowy pl&ik
menu-copy = &Kopiuj
menu-rename = &Zmień nazwę lub przenieś
menu-link = Utwórz &dowiązanie
menu-multirename = Zmień nazwy według wz&orca…
menu-multirename-undo = Cofnij ostatnią zbiorczą zmianę nazw
menu-mkdir = U&twórz folder
menu-delete = &Usuń
menu-wipe = &Wymaż
menu-checksum = &Sumy kontrolne…
menu-checksum-verify = Sprawdź według listy
menu-split = Podziel plik…
menu-combine = Połącz pliki
menu-delete-permanent = Usuń trwa&le
menu-archive-add = Dod&aj do archiwum
menu-archive-extract = &Rozpakuj archiwum
menu-archive-commands = Polecenia arc&hiwizera
menu-archive-test = Testuj archiwum
menu-attributes = Atry&buty pliku
menu-apply-command = Zastosuj pole&cenie
menu-replace-in-files = Zamień w plikach…
menu-select-group = Zaznacz &grupę
menu-select-ext = Zaznacz według tego rozszerzenia
menu-unselect-ext = Odznacz według tego rozszerzenia
menu-unselect-group = Odz&nacz grupę
menu-invert-selection = Odwróć zaznaczenie
menu-select-all = Zaznacz wszystko
menu-restore-selection = Odtwórz zaznaczenie
menu-dirsize = Rozmiar folderu
menu-find-file = &Znajdź plik
menu-history = &Historia poleceń
menu-view-history = Historia &podglądu i edycji
menu-folders-history = Historia &folderów
menu-usage = Użycie &dysku
menu-jobs = Zadan&ia
menu-branch = Widok gałęzi
menu-bookmarks = Skróty folderów
menu-swap-panels = Z&amień panele
menu-sync-panel = S&ynchronizuj drugi panel
menu-panels-onoff = &Włącz/wyłącz panele
menu-compare-folders = Porówna&j foldery
menu-sync-folders = Sy&nchronizuj foldery
menu-compare-contents = Po&równaj według zawartości
menu-panelize = Wyjście polecenia do panelu
menu-view-filtered = Podgląd przez polecenie
menu-learn-keys = Naucz klawiszy
menu-tunnel = Otwórz &tunel…
menu-tunnels = Tun&ele…
tunnel-title = Tunel: [lokalny:]host:port
tunnel-list-title = Tunele
tunnel-none = żaden tunel nie jest otwarty
tunnel-open = tunel otwarty: { $what }
tunnel-closed = tunel zamknięty
tunnel-failed = { $what }: { $why }
tunnel-needs-connection = tunel biegnie przez połączenie: najpierw otwórz je przez Alt+N
menu-connections = Połączenia
menu-disconnect = Rozłącz
menu-user-menu = &Menu użytkownika
menu-associations = Przypisania p&lików
menu-folder-shortcuts = &Skróty folderów
menu-plugin-commands = Pole&cenia wtyczek
menu-screens-list = Lista ekran&ów
menu-macro-record = Nagraj makro
menu-macro-list = Makra
menu-task-list = Lista zadań
menu-system-settings = Ustawienia &systemowe
menu-panel-settings = Ustawienia &panelu
menu-interface-settings = Ustawienia &interfejsu
menu-languages = &Język programu
menu-themes = &Motywy
menu-confirmations = P&otwierdzenia
menu-viewer-settings = Ustawienia pod&glądu
menu-editor-settings = Ustawienia &edytora
menu-codepages = Strony ko&dowe
menu-sort-groups = G&rupy sortowania
menu-mask-groups = Maski p&lików
menu-settings-all = Wszystkie ust&awienia
menu-highlighting = &Wyróżnianie plików
menu-plugins-config = Kon&figuracja wtyczek
menu-help = Pomoc
menu-save-setup = &Zapisz ustawienia
menu-quit = Ko&niec
opt-natural-sort = Naturalny porządek liczb
opt-allow-reverse = Pozwól na sortowanie odwrotne
opt-select-folders = Maski zaznaczają także foldery
cmd-no-network-name = Ścieżka na tym dysku nie ma nazwy sieciowej
opt-trash = Używaj systemowego Kosza
opt-system-copy = Używaj systemowej procedury kopiowania
opt-save-history = Zachowuj historię między sesjami
opt-history-lifetime = Czas życia historii, w dniach
opt-forever = bez końca
opt-fetch-limit = Limit rozmiaru plików zdalnych
opt-keepalive = SSH keep-alive
opt-off = wyłączone
opt-show-hidden = Pokazuj pliki ukryte i systemowe
opt-column-titles = Wyświetl tytuły kolumn
opt-status-line = Wyświetl linię statusu
opt-scrollbar = Wyświetl suwak
opt-free-space = Pokazuj wolne miejsce
opt-auto-refresh = Odświeżaj panel automatycznie
opt-dirs-first = Najpierw foldery
opt-selected-first = Zaznaczone najpierw
opt-reverse = Porządek odwrotny
opt-view = Widok panelu
opt-sort = Sortuj według
opt-quick-search = Szybkie szukanie
opt-quick-alt = z Alt
opt-quick-plain = samymi literami
opt-key-bar = Wyświetl pasek klawiszy
opt-status-bar = Wyświetl pasek stanu
opt-syntax = Koloruj rozpoznane języki
opt-markdown = Markdown złożony do czytania
opt-table = Otwieraj .csv i .jsonl jako tabelę
opt-keymap = Układ klawiszy
opt-clock = Zegar w rogu
opt-mouse = Obsługa myszy
opt-clipboard = Pozwól terminalowi przenosić schowek (OSC 52)
opt-command-screen = Pokazuj wyjście polecenia w trakcie jego działania
opt-menu-always = Zawsze pokazuj pasek menu
opt-save-exit = Zapisuj ustawienia przy wyjściu
opt-confirm-delete = Usuwanie
opt-confirm-overwrite = Zastępowanie
opt-confirm-exit = Wyjście
opt-confirm-editor = Wyjście z edytora z niezapisanymi zmianami
opt-applied = ustawienia zastosowano
opt-saved = zapisano w { $path }
opt-set-title = { $key }
opt-set = { $key } = { $value }
opt-not-a-toggle = To nie jest ustawienie tak/nie: Enter pozwala wpisać wartość
opt-settings-hint = ↑↓ wybór · pisz, aby zawęzić · Enter lub F4 wartość · Space przełącz · Del domyślna · Esc zamknij
opt-language-set = język: { $name }
sort-name = Nazwa
sort-ext = Rozszerzenie
sort-size = Rozmiar
sort-mtime = Czas zapisu
sort-btime = Czas utworzenia
sort-atime = Czas użycia
sort-descr = Opis
sort-owner = Właściciel
sort-git = Stan Git
key-sort-descr = Opis
key-sort-owner = Właśc
sort-none = Bez sortowania
sort-set = sortowanie: { $name }
btn-ok = OK
pick-hint = ↑↓ wybór · pisz, aby filtrować · Enter zastosuj · Esc zamknij
pick-empty = (nie ma z czego wybierać)
# ------------------------------------------------- the three histories (SPEC 06 §6)
hist-pinned = { $line } jest przypięte · nic go już nie wypchnie
hist-unpinned = { $line } nie jest już przypięte
hist-is-pinned = { $line } jest przypięte · Ins najpierw je odpina
hist-clear-body = Opróżnić tę historię ze wszystkiego, co nie jest przypięte?
hist-cleared = w historii zostało tylko to, co przypięte
history-hint = ↑↓ wybór · Enter zastosuj · Ins przypnij · Shift+Del jeden · Del wszystkie · Esc zamknij


# ------------------------------------------------- session editor (SPEC 10 §3.5)
site-new = Nowe połączenie
site-edit = Edycja połączenia
site-name = Nazwa
site-host = Host
site-port = Port
site-user = Użytkownik
link-title = Utwórz dowiązanie
link-name = Dowiązanie
link-kind = Rodzaj
link-kind-relative = symboliczne, względne — przetrwa przeniesienie drzewa
link-kind-absolute = symboliczne, bezwzględne — przetrwa przeniesienie dowiązania
link-kind-hard = trwałe — druga nazwa tego samego pliku
menu-colors = &Kolory…
palette-title = Kolory motywu { $theme }
palette-hint = Enter kolor · F2 zapisz · Esc zamknij
palette-ask = Kolor dla { $key }
palette-bad-color = { $value } nie jest kolorem (nazwa, 0–255 lub #rrggbb)
palette-saved = kolory zapisano w motywie { $theme }
palette-unsaved = kolory zmieniono tylko na ekranie — F2 je zapisuje
opt-viewer-status-line = Pokazuj górny wiersz (nazwa i pozycja)
opt-viewer-scrollbar = Wyświetl suwak
opt-viewer-key-bar = Wyświetl pasek klawiszy
opt-viewer-wrap = Długie wiersze
opt-viewer-wrap-off = nie zawijaj
opt-viewer-wrap-char = zawijaj na krawędzi
opt-viewer-wrap-word = zawijaj całe słowa
opt-viewer-tab-width = Rozmiar tabulatora
opt-viewer-remember = Pamiętaj pozycję i stronę kodową każdego pliku
opt-editor-tabs = Tabulatory
opt-editor-tabs-keep = zostaw bez zmian
opt-editor-tabs-typed = rozwijaj wpisywane
opt-editor-tabs-all = rozwijaj wszystkie przy otwieraniu pliku
opt-editor-tab-width = Rozmiar tabulatora
opt-editor-auto-indent = Automatyczne wcięcia
opt-editor-whitespace = Pokazuj spacje i tabulatory
opt-editor-cursor-after = Kursor staje za tekstem znalezionym przez szukanie
opt-editor-beyond-eol = Kursor może wyjść poza koniec wiersza
opt-editor-trim-trailing = Usuwaj końcowe spacje przy zapisie
editor-not-a-character = ten klawisz nie ma znaku do wstawienia
paste-one-line =
    { $count ->
        [one] wklejono tylko pierwszy wiersz · pominięto jeszcze jeden wiersz
        [few] wklejono tylko pierwszy wiersz · pominięto jeszcze { $count } wiersze
       *[other] wklejono tylko pierwszy wiersz · pominięto jeszcze { $count } wierszy
    }
clip-names =
    { $count ->
        [one] jedna nazwa w schowku
        [few] { $count } nazwy w schowku
       *[other] { $count } nazw w schowku
    }
clip-paths =
    { $count ->
        [one] jedna ścieżka w schowku
        [few] { $count } ścieżki w schowku
       *[other] { $count } ścieżek w schowku
    }
clip-line = wiersz poleceń jest w schowku
clip-selection = zaznaczenie jest w schowku
clip-off = schowek jest wyłączony — `ui.clipboard` w ustawieniach
history-gone = { $name } już nie istnieje
hl-new = Nowa grupa kolorów
hl-edit = Grupa kolorów
# ------------------------------------------------- the diff screen (SPEC 05 §3)
diff-title = Różnice
menu-diff = Po&każ różnice między plikami
diff-needs-two = Ustaw kursor na pliku w każdym panelu
diff-not-here = { $name } nie jest plikiem na tym dysku
diff-too-big = { $name } jest większy niż { $most }
diff-same = Oba pliki są takie same
diff-where = blok { $at } z { $count } · wierszy: -{ $removed } +{ $added }
diff-no-next = Ostatnia różnica
diff-no-previous = Pierwsza różnica
diff-hint = Alt+↑↓ bloki · Tab strona · F5 weź · F2 zapisz · U jednolity · Shift+F2 łatka · W C E ignoruj · Esc
diff-hint-git = Alt+↑↓ bloki · S dodaj blok do indeksu · Tab strona · F5 weź · F2 zapisz · Esc zamknij
diff-hint-git-staged = Alt+↑↓ bloki · S usuń blok z indeksu · Tab strona · F5 weź · F2 zapisz · Esc zamknij
diff-stage-edited = porównanie ma niezapisane scalenia: najpierw F2, potem S
diff-merged-left = Lewy plik przejął tę różnicę
diff-merged-right = Prawy plik przejął tę różnicę
diff-no-hunk-here = Na górze ekranu nie ma różnicy — Alt+↓ ją tam ustawi
diff-nothing-to-write = Niczego nie scalono
diff-written = zapisano: { $count }
diff-unwritten = Scalenie nie zostało zapisane — F2 je zapisuje, ponowny Esc je porzuca
diff-ignore-space-on = Odstępy ignorowane
diff-ignore-space-off = Odstępy znów się liczą
diff-ignore-case-on = Wielkość liter ignorowana
diff-ignore-case-off = Wielkość liter znów się liczy
diff-ignore-eol-on = Końce wierszy ignorowane
diff-ignore-eol-off = Końce wierszy znów się liczą

grab-hint = ↑↓←→ ruch · Shift+↑↓←→ zaznacz · Ctrl+A wszystko · Enter kopiuj · Esc zamknij
grab-copied = skopiowano wierszy ekranu: { $rows }

# ------------------------------------------------- the code pages (SPEC 05 §1.12)
cp-title = Strony kodowe
cp-name-title = Jak nazwać tę stronę kodową
cp-set = Odczyt jako { $name }
menu-panel-names = Strona kodowa nazw…
names-title = Czytaj nazwy jako
names-system = Tak, jak podaje je system
names-set = Nazwy czytane jako { $name }
names-system-set = Nazwy tak, jak podaje je system
names-not-here = W tym systemie nazwa nie jest ciągiem bajtów, więc nie da się jej odczytać inaczej
cp-marked = { $name } jest teraz wśród tych, po których chodzi F8
cp-unmarked = { $name } nie jest już wśród tych, po których chodzi F8
cp-hint = ↑↓ wybór · Enter czytaj tak · Ins zaznacz dla F8 · F4 zmień nazwę · Esc zamknij

# ------------------------------------------------- the named mask groups (SPEC 02 §5b)
mg-title = Grupa masek
mg-name = Nazwa
mg-mask = Maski
mg-hint = Ins dodaje grupę, Del ją usuwa, F4 edytuje · `·` oznacza trzy wbudowane
mg-saved = <{ $name }> obowiązuje; „Zapisz ustawienia” ją zapisze
mg-removed = <{ $name }> usunięto
mg-bad = <{ $name }>: { $problem }
mg-needs-name = nazwa bez spacji, przecinków i nawiasów ostrych
mg-needs-mask = grupa, która nic nie oznacza, do niczego nie pasuje
mg-built-in = <{ $name }> to jedna z trzech wbudowanych · F4 nadaje jej inne maski

sg-title = Grupa sortowania
sg-name = Nazwa
sg-mask = Maski
sg-place = Gdzie
sg-place-front = przed resztą
sg-place-back = za resztą
sg-hint = Ins dodaje grupę, Del ją usuwa, F4 edytuje, Ctrl+↑↓ przesuwa
sg-saved = Grupa obowiązuje; „Zapisz ustawienia” ją zapisze
sg-removed = { $name } usunięto
sg-bad = { $name }: { $problem }
sg-needs-mask = grupa bez maski niczego by nie zawierała
hl-name = Nazwa
hl-mask = Maska lub maski
hl-kind = Dotyczy
hl-kind-both = plików i folderów
hl-kind-file = tylko plików
hl-kind-dir = tylko folderów
hl-size = Rozmiar (1M-10M, -4K, 1G-)
hl-age = Wiek w dniach (<=1, >=30, 7-30)
hl-date-field = Wiek liczony według
hl-attrs = Atrybuty (h r x s b, - oznacza „nie może mieć”)
hl-fg = Kolor tekstu (#d7875f, red, file.archive)
hl-bg = Tło, zapisywane tak samo
hl-hint = ↑↓ wybór · Enter/F4 edytuj · Ins nowa · Del usuń · Ctrl+↑↓ przesuń · Esc zamknij
hl-saved = grupa obowiązuje · Opcje → Zapisz ustawienia zapisze ją do pliku
hl-removed = { $name } usunięto · Opcje → Zapisz ustawienia zapisze to do pliku
hl-bad = { $name }: { $problem }
filter-off = Bez filtra: wszystkie nazwy
filter-hint = ↑↓ wybór · Enter użyj · F4 edytuj · Ins nowy · Del usuń · Esc zamknij
filter-new = Nowy filtr
filter-edit = Filtr
filter-name-hint = z nazwą trafia do menu filtrów; bez nazwy jest używany jeden raz
filter-applied = filtr włączony: { $name }
filter-cleared = filtr wyłączony
filter-saved = { $name } działa w panelu i jest w menu filtrów · Opcje → Zapisz ustawienia zapisze go do pliku
filter-removed = { $name } usunięto · Opcje → Zapisz ustawienia zapisze to do pliku
editor-no-block = nic nie jest zaznaczone
editor-unique = usunięto powtórzonych wierszy: { $count }
editor-no-bracket = nie ma tu nawiasu albo nic do niego nie pasuje
editor-tabs-expanded = tabulatory rozwinięto w wierszach: { $lines } — zapis to utrwali
site-protocol = Protokół
site-protocol-sftp = SFTP (przez SSH)
site-protocol-ftp = FTP
site-protocol-ftps = FTP z TLS
site-protocol-dav = WebDAV
site-protocol-davs = WebDAV przez HTTPS
site-auth = Uwierzytelnianie
site-auth-agent = agent SSH
site-auth-key = plik klucza
site-auth-ask = pytaj o hasło
site-key-file = Plik klucza
site-start-dir = Folder zdalny
site-group = Grupa
site-keepalive = Keep-alive (s)
site-compress = Kompresja
site-page-session = Sesja
site-page-environment = Środowisko
site-page-directories = Foldery
site-page-sftp = SFTP
site-password-note = Hasło nie jest zapisywane: pytanie o nie pojawia się przy łączeniu.
site-env-eol = Tłumacz końce wierszy (jeszcze nie)
site-env-utf8 = Wymuś nazwy w UTF-8 (jeszcze nie)
site-jump = Host pośredni
site-jump-note = Połączenie przez ten host: host, user@host lub user@host:port. Puste oznacza prosto do serwera.
site-sftp-ciphers = Wybierz szyfry (jeszcze nie)
btn-connect = Połącz
site-needs-host = połączenie potrzebuje hosta
site-needs-key = wybierz plik klucza albo inny sposób uwierzytelniania
site-bad-port = `{ $port }` nie jest liczbą
site-stored = połączenie zapamiętane: { $name } (Shift+F9, aby je zapisać)
# ------------------------------------------------- structure pane (SPEC 05 §1.8)
key-struct = Strukt
key-markdown = MD
key-table = Tabela
key-filter = Filtr
key-decompile = Dekomp
key-next = Następ
key-prev = Poprz
key-codepages = StrKod
key-goto = IdźDo
key-to-panel = Panel
key-blame = Blame
viewer-structure = Struktura
viewer-structure-hint = ↑↓ wiersz · Enter idź do · Tab zawartość · Shift+F11 zamknij
viewer-structure-hint-tab = Tab przenosi klawisze tutaj
viewer-no-structure = nie ma czego dekodować: panel struktury nie zna tego formatu
viewer-no-target = ten wiersz nie wskazuje żadnego miejsca w pliku
viewer-form-opened = Formularz jako tekst, tak jak pokazuje go Delphi. F4 wraca do bajtów.
viewer-metadata-opened = Cały tekst, złożony do czytania. F4 wraca do bajtów.
viewer-went-to = przejście do { $offset }
viewer-in-panel = panel stoi na { $name }
viewer-not-in-panel = { $name } nie ma w tym panelu
viewer-no-more-files = w panelu nie ma więcej plików
editor-unsaved-for-viewer = najpierw zapisz: podgląd czyta plik z dysku
# ------------------------------------------------- command line (SPEC 06 §7)
cmd-running = trwa: { $command }
prompt-panelize = Polecenie, którego wyjście wymienia pliki
prompt-viewer-filter = Polecenie, którego wyjście czytać zamiast pliku
learn-title = Jakie klawisze wysyła ten terminal
learn-hint = Naciskaj klawisze. Podświetlona komórka oznacza, że klawisz dotarł; dwa razy Esc — wyjście.
learn-last = ostatni: { $key } — { $command }
learn-unbound = do niczego nieprzypisany
recode-title = Zapisz ponownie jako
recode-what = plików: { $count }
recode-from = Czym są teraz
recode-to = Zapisz je jako
recode-auto = ustal dla każdego pliku
recode-eol = Końce wierszy
recode-eol-keep = zostaw bez zmian
recode-working = ponowny zapis plików: { $count }
recode-done = zapisano: { $changed }, już zgodnych: { $same }, odrzuconych: { $failed }
menu-recode = Zapisz ponownie jako…
panelize-title = { $command }
panelize-done = plików: { $count }, pominiętych wierszy: { $dropped }
panelize-nothing = { $command }: żaden wiersz nie wymienił istniejącego pliku
cmd-interrupted = do polecenia wysłano przerwanie
cmd-done = { $command } zakończone
cmd-failed = { $command } zakończyło się kodem { $code }
cmd-busy = jakieś polecenie już działa
shell-not-kept = { $shell } nie oznaczył znaku zachęty na czas: każde polecenie działa teraz osobno, jak wcześniej
prompt-apply = Polecenie do uruchomienia dla każdego z plików ({ $count }):
cmd-apply-no-ask = polecenia, które pyta o wartość, nie da się stosować plik po pliku
cmd-apply-progress = { $command }  ({ $done } z { $total })
cmd-apply-stopped = zatrzymano po { $done } z { $total }: { $command } nie powiodło się
cmd-not-remote = polecenie potrzebuje folderu lokalnego; ten panel jest na serwerze
cmd-no-history = historia poleceń jest pusta
cmd-no-previous = nie ma dokąd wrócić
cmd-no-such-dir = { $path }: nie ma takiego folderu
cmd-no-such-path = { $path }: nie ma takiego pliku ani folderu
cmd-no-selection = nic nie jest zaznaczone
cmd-condition-false = nie uruchomiono: { $command }
cmd-screen-cleared = ekran wyczyszczony
cmd-stack-empty = stos folderów jest pusty
cmd-stack-cleared = stos folderów wyczyszczony
cmd-pushed = folderów na stosie: { $count }
cmd-environment = środowisko
cmd-set = { $name } = { $value }
cmd-not-set = { $name } nie jest ustawione
user-screen-title = Wyjście — wierszy: { $lines }, od wiersza { $at }
user-screen-empty = (nic jeszcze nie uruchomiono)
viewer-no-branch = ten wiersz nie ma adresu, za którym można pójść
viewer-no-back = nie ma do czego wracać

# ------------------------------------------------- find files (SPEC 06 §1)
folders-title = Znajdź folder
folders-reading = Odczytywanie folderów…
folders-read = folderów: { $count }
folders-capped = folderów: { $count }, do limitu
folders-local-only = lista folderów dotyczy tego komputera; serwer ma swoją historię i listę skrótów
find-title = Znajdź plik
find-mask = Maska lub maski plików
find-size = Rozmiar (100M, 1M-1G, -4K)
find-text = Zawierające tekst
find-case = Uwzględnij wielkość liter
find-words = Tylko całe słowa
find-hex = Tekst powyżej to wzorzec szesnastkowy
find-not = Pliki, które go NIE zawierają
find-scope = Szukaj w
find-scope-panel = tym folderze
find-scope-tree = tym folderze i poniżej
find-scope-selection = zaznaczonych folderach
find-scope-root = od katalogu głównego tego napędu
find-scope-other = drugim panelu i poniżej
find-links = Podążaj za linkami symbolicznymi
find-dirs = Foldery także w wynikach
find-archives = Szukaj w archiwach
find-results = Znaleziono { $count } · przejrzano plików: { $files }, folderów: { $dirs }
find-done = znaleziono { $count }, przejrzano plików: { $files } · { $elapsed }
find-none = nic nie znaleziono
find-stopped = zatrzymano po przejrzeniu plików: { $files }
find-hint = ↑↓ wybór · Enter idź do · F3 podgląd · F4 edytuj · F5 do panelu · Esc zatrzymaj
find-hint-done = ↑↓ wybór · Enter idź do · F3 podgląd · F4 edytuj · F5 do panelu · Esc zamknij
find-panelised = wyników w panelu: { $count }
find-needs-mask = potrzebna jest maska: `*` oznacza każdy plik
find-bad-size = `{ $text }` nie jest rozmiarem — { $error }
# ------------------------------------------------- replace across files
replace-title = Zamień w plikach
replace-needle = Znajdź
replace-with = Zamień na
replace-case = Uwzględnij wielkość liter
replace-words = Tylko całe słowa
replace-pattern = Tekst powyżej to wyrażenie regularne
replace-hex = Oba to bajty szesnastkowe
replace-scope = zaznaczonych plików: { $count }
replace-needs-needle = nie ma czego szukać
replace-confirm-title = Zamień
replace-confirm-body = Zmieni się wystąpień: { $count }, w plikach: { $files }.
replace-nothing = nie ma czego zamieniać: żaden plik by się nie zmienił
replace-done = zamieniono wystąpień: { $count }, w plikach: { $files }

# ── archives (SPEC 04 §4) ───────────────────────────────────────────────────────────────
arc-writing-back = Zapisywanie { $name } z powrotem do archiwum
arc-move-out = aby wyjąć coś z archiwum, skopiuj to klawiszem F5
arc-add-files-only = pojedynczo dodaje się tylko pliki; Shift+F1 dodaje cały folder

rename-title = Zbiorcza zmiana nazw
rename-name = Nazwa
rename-ext = Rozszerzenie
rename-search = Znajdź
rename-replace = Zamień na
rename-regex = Wyrażenie regularne ($1 w zamienniku)
rename-case = Litery
rename-case-same = bez zmian
rename-case-lower = małe litery
rename-case-upper = WIELKIE LITERY
rename-case-title = Każde Słowo Wielką Literą
rename-case-sentence = Tylko pierwsza litera wielka
rename-counter = Licznik: początek, krok, cyfry
rename-fields = [N] nazwa  [E] rozszerzenie  [N1-3] część  [C] licznik  [d] [t] data, czas  [P] folder
rename-not-in-archive = nazwy wielu plików zmienia się w folderze, nie w archiwum: tam F6 zmienia nazwę jednego wpisu
rename-bad-counter = licznik to trzy liczby całkowite: początek, krok, cyfry; { $counter } nimi nie jest
rename-cannot = nie można zmienić nazwy: { $why }
rename-problem-empty = nie została żadna nazwa
rename-problem-invalid = plik nie może mieć takiej nazwy
rename-problem-twice = tę nazwę dostaje też inny plik
rename-problem-exists = plik o tej nazwie już istnieje
rename-more = … i jeszcze { $count }
rename-blocked =
    { $count ->
        [one] jeden plik nie może przyjąć nowej nazwy: nic nie zostanie zmienione, dopóki to niemożliwe
        [few] { $count } pliki nie mogą przyjąć nowych nazw: nic nie zostanie zmienione, dopóki to niemożliwe
       *[other] { $count } plików nie może przyjąć nowych nazw: nic nie zostanie zmienione, dopóki to niemożliwe
    }
rename-summary = zmienia się { $count } z { $total }
rename-nothing = żadna nazwa się nie zmienia
rename-nothing-to-undo = nie ma zbiorczej zmiany nazw do cofnięcia
rename-done =
    { $count ->
        [one] zmieniono nazwę jednego pliku · cofniesz to z menu Pliki
        [few] zmieniono nazwy { $count } plików · cofniesz to z menu Pliki
       *[other] zmieniono nazwy { $count } plików · cofniesz to z menu Pliki
    }
rename-undone =
    { $count ->
        [one] przywrócono jedną nazwę
        [few] przywrócono { $count } nazwy
       *[other] przywrócono { $count } nazw
    }
pack-title = Dodaj do archiwum
pack-name = Archiwum
pack-format = Format
pack-level = Kompresja
pack-solid = Bloki ciągłe (7z: mniejsze, wolniejszy odczyt jednego pliku)
pack-delete-after = Usuń pliki po spakowaniu
pack-level-normal = normalna
pack-level-fast = szybka
pack-level-best = najlepsza
pack-scope = zaznaczonych plików: { $count }
pack-needs-name = archiwum potrzebuje nazwy
pack-format-read-only = { $format } jest tu tylko czytany, nie zapisywany: wybierz inny format lub inną nazwę
pack-single-one-file = { $format } mieści jeden plik: wybierz jeden albo użyj { $tar } dla folderu lub wielu plików
unpack-title = Rozpakuj archiwum
unpack-to = Rozpakuj do
unpack-scope = z { $name }
unpack-not-an-archive = to nie jest archiwum, które umiemy czytać

# ── the special panel kinds (SPEC 02 §7) ────────────────────────────────────────────────
panel-quick-view = Szybki podgląd
panel-info = Informacja
panel-tree = Drzewo
compare-done = różnią się: { $left } po lewej, { $right } po prawej
cmp-subdirs = Wchodź do podfolderów
cmp-depth = Jak głęboko schodzić (0 = do samego dołu)
cmp-trees-title = Co się różni
cmp-trees-working = porównywanie drzew: plików { $seen }, odczytano { $read }
cmp-trees-done = różnych: { $differ }, tylko po lewej: { $left }, tylko po prawej: { $right }, wszystkich par: { $pairs }
cmp-trees-same = oba drzewa są takie same (par: { $pairs }, sprawdzono: { $same })
menu-duplicates = Znajdź duplikaty
dup-title = Duplikaty plików
dup-working = szukanie duplikatów: plików { $seen }, odczytano { $hashed }
dup-done = grup kopii: { $groups }, plików: { $files }, zmarnowane { $wasted }
dup-none = nic tu nie jest kopią czegoś innego (plików: { $seen })
dup-stopped = zatrzymano po przejrzeniu plików: { $seen }
dup-partial = zatrzymano przed końcem
cmp-title = Porównaj
cmp-case = Dopasowuj nazwy z uwzględnieniem wielkości liter
cmp-size = Inny rozmiar to różnica
cmp-time = Inny czas to różnica
cmp-seconds = Czasy różniące się o tyle sekund to ta sama chwila
cmp-contents = Czytaj bajty
cmp-when-time = Czytaj je tylko tam, gdzie czasy się różnią
cmp-fragment = Czytaj tylko tyle pierwszych KiB (0 = całość)
cmp-ignore-eol = Same końce wierszy nie są różnicą
cmp-selected = Tylko zaznaczone
cmp-stop = Zatrzymaj się na pierwszej różnicy
cmp-newer = Zaznaczaj tylko nowszą kopię
menu-compare-setup = Porównaj z &opcjami
compare-contents-eol-ignored = różnią się tylko końcami wierszy, zignorowano: { $count }
compare-contents-trusted = uznano za takie same bez czytania: { $count }
compare-contents-working = Porównywanie zawartości: { $done } z { $total }
compare-contents-done = różnych par: { $differ } z { $pairs }
compare-contents-same = wszystkie pary ({ $pairs }) są identyczne
compare-contents-none = żaden plik nie ma tej samej nazwy po obu stronach
compare-contents-local = porównanie według zawartości wymaga obu paneli w lokalnym systemie plików
compare-contents-eol = tylko końce wierszy: { $count }
compare-contents-only = tylko po jednej stronie: { $count }
compare-contents-unread = nie udało się odczytać: { $count }
compare-contents-stopped = porównywanie zatrzymane
quick-view-directory = { $name } to folder — F3 go mierzy
quick-view-not-here = bajtów tu nie ma: F3 najpierw je sprowadzi
info-path = Ścieżka
info-kind = Rodzaj
info-kind-local = folder na tym komputerze
info-kind-remote = na { $host }
info-kind-archive = wewnątrz { $name }
info-kind-list = lista plików
info-device = Urządzenie
info-folder = Folder
info-device-value = { $total }, wolne { $free }, klastry { $cluster }
info-slack = Straty na klastrach
info-filesystem = System plików
info-mount = Zamontowano
info-git = Repozytorium
info-git-clean = zgodne z indeksem
info-git-differing = { $count } z { $tracked } różni się od indeksu
info-git-uncounted = nie policzono
info-git-merge = trwa scalanie
info-git-rebase = trwa rebase
info-git-cherry-pick = trwa cherry-pick
info-git-revert = trwa revert
info-git-bisect = trwa bisect
info-memory = Pamięć
info-memory-value = wolne { $free } z { $total }
info-memory-total = { $total }
info-contents = Zawiera
info-contents-value = plików: { $files }, folderów: { $dirs }, { $bytes }
info-selected = Zaznaczono
info-selected-value = plików: { $count }, { $bytes }
info-cursor = Kursor
info-size = Rozmiar
info-modified = Zmieniono
info-description = Opis
key-mark = Zazn
key-asm = Asm
key-block = Blok
key-screens = Ekrany
key-site-edit = EdSesję
key-site-new = NowaSes
key-site-forget = UsuńSes
key-ai-edit = Edytuj
key-ai-new = Nowa
key-ai-remove = Usuń
key-ai-forget = Zapomn
key-ai-threads = Histor
hex-needs-a-file = ten bufor nie ma pliku, z którego można czytać bajty
hex-save-first = najpierw zapisz: dwa edytory nie mogą trzymać nawzajem swoich niezapisanych zmian

# ── the hex editor's block operations (SPEC 05 §2.6) ────────────────────────────────────
hex-block-title = Blok
hex-op-write = Zapisz { $size } do pliku
hex-op-read = Wczytaj plik w pozycji { $at }
hex-op-insert = Wstaw bajty zerowe w pozycji { $at }
hex-op-copy = Skopiuj bajty z innego miejsca do { $at }
hex-op-move = Przenieś bajty z innego miejsca do { $at }
hex-op-fill = Wypełnij { $size } bajtem
hex-op-xor = Wykonaj alternatywę wykluczającą na { $size } z kluczem
hex-op-delete = Usuń { $size } — { $was } zmieni się w { $becomes }
hex-write-title = Zapisz blok do
hex-read-title = Który plik wczytać
hex-fill-title = Którym bajtem wypełnić (hex)
hex-xor-title = Alternatywa wykluczająca z którym kluczem (hex)
hex-insert-title = Ile bajtów zerowych
hex-copy-title = Skąd kopiować — przesunięcie i długość (hex)
hex-move-title = Skąd przenieść — przesunięcie i długość (hex)
hex-not-hex = to nie jest ciąg cyfr szesnastkowych
hex-not-a-count = to nie jest liczba bajtów
hex-not-a-range = to nie jest przesunięcie i długość w hex
hex-range-outside = ten zakres leży poza plikiem ({ $size })
hex-move-into-itself = bloku nie można przenieść do wnętrza samego siebie
hex-filled = wypełniono { $size }
hex-xored = na { $size } wykonano alternatywę wykluczającą — to nie jest szyfrowanie
hex-written = { $size } zapisano do { $name }
hex-read-done = { $size } wczytano z { $name }
hex-inserted = wstawiono zer: { $size }
hex-deleted = usunięto { $size }
hex-device = { $name }: urządzenie o rozmiarze { $size }, otwarte tylko do odczytu
asm-title = Asembluj w { $at } ({ $arch })
asm-no-arch = { $arch } nie jest architekturą, dla której umiemy asemblować
asm-written = { $text } — { $bytes }
asm-written-longer = { $text } — { $bytes } (o { $grew } bajtów dłużej)
screens-title = Ekrany
screens-panels = Panele
quit-title = Wyjście z FinalCommander
quit-body = Zakończyć?
screens-quit-title = Niezapisane zmiany
screens-quit-body = Te ekrany zawierają zmiany, których nikt nie zapisał:
macro-recording = Nagrywanie makra · Ctrl+. je zatrzymuje
macro-empty = nic nie nagrano
macro-dropped = nagranie odrzucono
macro-key-title = Który klawisz
macro-key-body = Nagrano kroków: { $steps }. Naciśnij klawisz, na który to makro ma reagować, albo Esc, aby je odrzucić.
macro-describe = Do czego służy makro pod { $key }
macro-saved = makro pod { $key } zapisane, kroków: { $steps }
macro-none = nie nagrano jeszcze żadnego makra · Ctrl+. nagrywa makro
macro-list-title = Makra
macro-play-title = Jak ma zostać odtworzone
macro-play-once = Raz, od miejsca, w którym stoi kursor
macro-play-each = Raz dla każdego zaznaczonego pliku
macro-sure-title = Odtwórz to makro
macro-sure-body = Makro pod { $key } kopiuje, przenosi, usuwa lub nadpisuje. Odtworzyć je?
macro-too-deep = makro nie może odtwarzać samego siebie bez końca
plugin-list-title = Polecenia wtyczek
plugin-none-title = Nie zainstalowano wtyczek
plugin-none-open = Otwórz folder wtyczek ({ $dir })
plugin-none-help = Jak działają wtyczki i gdzie je znaleźć
plugin-help-heading = Wtyczki
plugin-why-off = wyłączona w Opcje → Konfiguracja wtyczek
plugin-why-panel = tylko w panelach
plugin-why-viewer = tylko w podglądzie
plugin-why-editor = tylko w edytorze
plugin-why-local = wymaga folderu na dyskach tego komputera
plugin-why-files = wymaga pliku pod kursorem lub zaznaczonego
plugin-why-masks = tylko dla { $masks }
plugin-why-bad-mask = jej maska jest błędna: { $error }
plugin-why-here = wymaga { $names } w tym folderze
plugin-why-inside = wymaga { $names } tutaj lub w folderze powyżej
plugin-cannot = { $label }: { $why }
plugin-running = { $label }…
plugin-on-selected = dla zaznaczonych plików: { $count }
plugin-failed = { $id }: { $why }
plugin-unknown-command = wtyczka poprosiła o `{ $id }`, a takiego polecenia nie ma
hex-copied = { $size } skopiowano z { $from }
hex-moved = { $size } przeniesiono z { $from }
menu-calculator = Kalkulator
menu-terminal = Terminal
calc-title = Kalkulator
calc-decimal = dziesiętnie
calc-hex = szesnastkowo
calc-octal = ósemkowo
calc-binary = dwójkowo
calc-characters = znaki

# The archive commands (Shift+F3) and the two parts of the order that are not the key.
arc-commands-title = Polecenia archiwizera
arc-password = Hasło do { $name }
arc-op-test = Testuj { $name }
arc-op-extract-here = Rozpakuj { $name } tutaj
arc-op-extract-to = Rozpakuj { $name } do…
arc-op-delete = Usuń { $name }
arc-op-add = Dodaj pliki do archiwum…
job-test = Testowanie archiwum
msg-dirs-first-on = najpierw foldery
msg-dirs-first-off = foldery razem z plikami
msg-selected-first-on = zaznaczone pliki najpierw
msg-selected-first-off = zaznaczone pliki po kolei

# The change-drive dialog (Alt+F1, Alt+F2): the type column, and its footer.
drive-fixed = Dysk
drive-removable = Wyjmowalny
drive-optical = Optyczny
drive-network = Sieciowy
drive-memory = Pamięć
drive-home = Domowy
drive-connection = Połączenie
drive-objects = Magazyn obiektów
drive-netbox = NetBox: zapisane sesje
drive-processes = Lista procesów
drive-temp = Panel tymczasowy
temp-added = dodano do panelu tymczasowego: { $count }
proc-none = listy działających procesów nie da się tu odczytać
drive-programs = Zainstalowane programy
drive-services = Usługi
svc-none = ten komputer nie ma menedżera usług, którego można zapytać
temp-not-a-file = wiersze tutaj nie są plikami: nie ma czego usuwać
svc-card = Usługa
svc-card-id = Nazwa
svc-card-name = Wyświetlana jako
svc-card-state = Stan
svc-card-description = Opis
svc-card-pid = Proces
svc-state-running = działa
svc-state-stopped = zatrzymana
svc-state-failed = awaria
svc-state-other = inny
key-service-start = Start
key-service-restart = Restart
key-service-stop = Stop
svc-ask-stop = Zatrzymać { $name }?
svc-ask-restart = Uruchomić ponownie { $name }?
btn-service-stop = Zatrzymaj
btn-service-restart = Uruchom ponownie
svc-working = { $name }: zapytanie do systemu…
svc-started = { $name } uruchomiono
svc-stopped = { $name } zatrzymano
svc-restarted = { $name } uruchomiono ponownie
svc-needs-root = { $name }: to wymaga uprawnień root — program nie poprosi o nie w twoim imieniu
svc-failed = { $name }: { $error }
prog-none = nic tu nie mówi, co jest zainstalowane
prog-card = Zainstalowany program
prog-no-uninstall = nic nie mówi, jak usunąć { $name }
prog-uninstall-ready = polecenie usuwające { $name } czeka w wierszu poleceń, nieuruchomione
prog-card-name = Nazwa
prog-card-version = Wersja
prog-card-publisher = Wydawca
prog-card-size = Rozmiar na dysku
prog-card-installed = Zainstalowano
prog-card-location = Folder
prog-card-uninstall = Usuwany poleceniem
drive-sockets = Lista gniazd
sock-none = nic tu nie nasłuchuje albo tabeli nie da się odczytać
sock-no-process = nic nie mówi, który proces trzyma to gniazdo
proc-card = Proces
proc-gone = { $name } już nie działa
proc-card-name = Nazwa
proc-card-pid = Identyfikator procesu
proc-card-parent = Uruchomiony przez
proc-card-user = Użytkownik
proc-card-state = Stan
proc-card-threads = Wątki
proc-card-memory = Pamięć
proc-card-mapped = Zmapowane
proc-card-bits = Bitowość
proc-card-started = Uruchomiono
proc-card-exe = Plik wykonywalny
proc-card-cwd = Folder
proc-card-command = Wiersz poleceń
proc-card-group = Grupa
proc-card-sid = SID
proc-card-priority = Priorytet
proc-card-session = Sesja
proc-card-uptime = Działa od
proc-card-cpu-user = Czas procesora, własny
proc-card-cpu-kernel = Czas procesora, w jądrze
proc-card-handles = Otwarte uchwyty
proc-card-faults = Błędy stron
proc-card-private = Pamięć prywatna
proc-card-swap = W pliku wymiany
proc-card-io-read = Odczytano
proc-card-io-write = Zapisano
proc-card-memory-peak = Szczyt zestawu roboczego
proc-card-private-resident = Zestaw roboczy, prywatny
proc-card-swap-peak = Szczyt pliku stronicowania
proc-card-virtual = Bajty wirtualne
proc-card-virtual-peak = Szczyt bajtów wirtualnych
proc-card-paged-pool = Pula stronicowana
proc-card-nonpaged-pool = Pula niestronicowana
proc-card-io-other = Inne we/wy
proc-card-io-read-ops = Operacje odczytu
proc-card-io-write-ops = Operacje zapisu
proc-card-io-other-ops = Inne operacje
proc-kill-title = Zakończ proces
proc-kill-ask = Zakończyć { $name } [{ $pid }]? Wszystko, czego nie zapisał, przepadnie.
proc-killing = Kończenie { $name }…
proc-killed = { $name } zakończony
proc-kill-failed = nie udało się zakończyć { $name }: { $why }
proc-card-env = Środowisko
proc-card-modules = Załadowane moduły
proc-card-version = Wersja
proc-card-description = Opis
drives-hint = ↑↓ wybór · 0-9 panele · Ctrl+1..9 kolumny · F9 wszystkie · Ctrl+R odśwież
drives-title = Zmień napęd: co pokazywać
drives-col-type = typ
drives-col-device = urządzenie
drives-col-label = etykietę
drives-col-fs = system plików
drives-col-size = rozmiary
drives-col-removable = napędy wyjmowalne
drives-col-panels = panele, które nie są napędami
drives-col-optical = napędy optyczne
drives-col-network = udziały sieciowe
drives-shown = pokazywane: { $what }
drives-hidden = ukrywane: { $what }
drives-reread = napędy odczytano ponownie
bookmarks-hint = Enter idź · Ins zapamiętaj · Del usuń · F4 nazwij · Esc zamknij

# The search dialog (F7), which asks the whole question in one place.
search-title = Szukaj
search-for = Znajdź
search-mode = Jako
search-mode-text = tekst
search-mode-hex = hex
search-mode-regex = wyrażenie regularne
search-case = Uwzględnij wielkość liter
search-words = Tylko całe słowa
btn-find-next = Znajdź następny
btn-find-prev = Znajdź poprzedni
# The help: the chapters' own names, and the note on a chapter nobody has translated yet
help-chapter-about = Co to jest
help-chapter-panels = Panele
help-chapter-files = Pliki: kopiowanie, przenoszenie, szukanie, archiwa
help-chapter-viewer = Podgląd
help-chapter-editor = Edytor, panel szesnastkowy i asembler
help-chapter-network = Serwery
help-chapter-config = Konfiguracja, klawisze i kolory
help-chapter-keys = Klawisze, tak jak są teraz przypisane
help-chapter-commands = Wszystkie polecenia i ich klawisze
help-chapter-options = Wszystkie ustawienia i ich wartości
help-here-panels = Panele
help-here-viewer = Podgląd
help-here-editor = Edytor
help-here-hex = Edytor szesnastkowy
help-here-dialog = To okno dialogowe
help-in-english = Ten rozdział nie został jeszcze przetłumaczony na twój język, więc dalej jest po angielsku. Klawisze i ustawienia, które wymienia, są takie same w każdym języku.
help-about-byline = FinalCommander { $version } — autor: Matteo Baccan
key-ok = OK
key-cancel = Anuluj
key-dialog-move = Przesuń
key-tree = Drzewo
key-config = Konfig
browse-local-only = plik wybiera się z tego komputera; ścieżkę na serwerze wpisuje się ręcznie, a Tab ją uzupełnia
key-browse = Przegl
table-prompt-filter = Wiersze zawierające
table-sorted = posortowano według { $column }
table-sorted-back = posortowano według { $column }, w odwrotnej kolejności
table-unsorted = z powrotem w kolejności z pliku
table-filtered = pozostało wierszy: { $rows }
table-unfiltered = znów wszystkie wiersze
table-cut-sorted = z pierwszych { $rows } wierszy, bo tylko tyle odczytano
table-too-many = za dużo rekordów, by je tu sortować lub filtrować
table-copied =
    { $rows ->
        [one] jeden wiersz w schowku
        [few] { $rows } wiersze w schowku
       *[other] { $rows } wierszy w schowku
    }
table-headed = pierwszy wiersz to nagłówek
table-not-headed = pierwszy wiersz to dane: kolumny nazywają się według numerów
table-named-fields = ten format sam nazywa swoje pola: nie ma nagłówka do zgadywania

# Splitting and combining (SPEC 03 §9)
split-title = Podziel plik
split-target = Do
split-size = Każda część
split-size-hint = 1440K, 700M, 4G — ostatnia część to reszta
split-count = Albo na
split-count-hint = części — liczba tutaj ma pierwszeństwo przed rozmiarem
split-bad-count = { $count } nie jest liczbą części: dwie lub więcej
split-local-file = Dzielenie i łączenie działają na pliku na tym komputerze
split-bad-size = „{ $size }” nie jest rozmiarem: zapisz go jako 700M lub 4G
combine-not-a-piece = To nie jest część: części podzielonego pliku kończą się na .001, .002, … albo mają obok plik .crc
pieces-working = Dzielenie lub łączenie: { $done } z { $total }
pieces-cancelled = Zatrzymano, a to, co zapisano, zostało usunięte
split-done = zapisano części: { $parts }, a obok nich { $list }
combine-done-checked = { $name } połączono z części: { $parts }, { $bytes }, i zgadza się ze swoim .crc
combine-done-unchecked = { $name } połączono z części: { $parts }, { $bytes } — nie było pliku .crc do sprawdzenia
pieces-failed = Nie wykonano: { $error }

# The branch view (SPEC 02 §3d)
branch-reading = Odczytywanie wszystkich plików w { $path }…
branch-title = Gałąź: { $path }
branch-done = plików: { $count }, ze wszystkich folderów poniżej tego · Ctrl+Alt+B wraca
branch-truncated = plików: { $count }, i na tym przeglądanie stanęło: gałąź jest większa, niż lista może sensownie pokazać
branch-not-here = Widok gałęzi dotyczy folderu, a ten panel jest listą

## The release check (SPEC 11 §5).
menu-check-update = Sprawdź ak&tualizacje
opt-update-check = Sprawdzaj nowe wydania raz dziennie
btn-open-page = Otwórz stronę
update-checking = Pytanie GitHuba o najnowsze wydanie…
update-title = Aktualizacje
update-available = Wyszedł FinalCommander { $version } (to jest { $current }).
update-available-status = Wyszedł FinalCommander { $version }: Opcje → Sprawdź aktualizacje
update-current = To jest najnowsze wydanie, { $version }.
update-failed = Nie udało się zapytać o najnowsze wydanie: { $why }

## Duplicate files (SPEC 06 §13).
menu-dupes = Znajdź duplikaty plików
dupes-title = Duplikaty plików
dupes-recursive = Szukaj w podfolderach
dupes-min-size = Najmniejszy plik
dupes-min-size-hint = Rozmiar zapisany tak jak w filtrach: 1, 100K, 5M. Puste pliki nigdy nie są porównywane.
dupes-select = Zaznacz wszystkie kopie oprócz najstarszej
btn-search = Szukaj
dupes-not-here = Szukanie duplikatów zaczyna się od folderu albo od zaznaczenia.
dupes-bad-size = To nie rozmiar: { $size }
dupes-looking = Szukanie duplikatów plików…
dupes-progress = Duplikaty: przebieg { $pass } z 3, { $done }/{ $total }
dupes-none = Wśród plików ({ $count }) nie ma duplikatów.
dupes-panel = Duplikaty: grup — { $groups }
dupes-done = grup duplikatów: { $groups }, w kopiach { $size }.
dupes-truncated = Zatrzymano przed końcem: jak dotąd grup { $groups }, w kopiach { $size }.

## SQLite queries (SPEC 04 §4b).
sql-empty = Wpisz zapytanie po sql:, na przykład sql: SELECT * FROM t
sql-no-database = sql: wymaga bazy danych SQLite: wejdź do niej albo ustaw na niej kursor.
sql-running = Wykonywanie zapytania…
sql-failed = Zapytanie nie powiodło się: { $why }

## Asking a language model (SPEC 06 §14).
ai-none = Nie skonfigurowano żadnej usługi AI: dodaj blok [[ai]] do fcmd.toml (name, url, model, api, token_env).
ai-title = Zapytaj AI
ai-no-thread = nie ma odpowiedzi, którą można kontynuować: najpierw zapytaj przez `ai:` albo ustaw kursor na odpowiedzi w folderze odpowiedzi
ai-nothing-to-explain = nie ma tu czego wyjaśniać: otwórz plik w podglądzie lub edytorze albo zapytaj zaraz po poleceniu, które się nie powiodło
ai-explain-question = Co to jest, z { $name }? Wyjaśnij krótko.
ai-tokens = tokenów: { $in } na wejściu, { $out } na wyjściu
ai-none-panel = Nie ma jeszcze usługi AI — F7 ją dodaje
ai-service-removed = usunięto { $name }
ai-probing = Logowanie do { $name } na { $host }…
ai-probe-ok = { $host } odpowiedział HTTP { $status } w { $ms } ms, modeli: { $models } — wpisz pytanie i naciśnij Enter
ai-probe-no-model = (modelu { $model } wśród nich nie ma)
ai-row-context = kontekst
ai-row-conversation = rozmowa
ai-session-title = AI: { $name } · { $model } — plików w kontekście: { $files }
ai-context-added = { $count ->
        [one] Jeden plik
        [few] { $count } pliki
       *[other] { $count } plików
    } w kontekście
ai-context-nothing = Nie ma czego dodać: do kontekstu trafiają tylko pliki z tego dysku
ai-command-ready = polecenie jest w wierszu — przeczytaj je, zanim naciśniesz Enter
ai-commit-question = Napisz komunikat commita dla zmian dodanych do indeksu.
ai-staged-diff = diff zmian z indeksu
ai-send-diff = Diff zmian z indeksu trafia do { $host }
ai-send-files = Plików: { $count }, { $size } — zostaną wysłane do { $host } razem z pytaniem.
ai-asking = Pytanie do { $name }…
ai-busy = { $name } myśli… { $secs } s · Esc zatrzymuje
ai-conversation-new = Nowa rozmowa: zacznie ją następne pytanie.
ai-live-empty = Wpisz pytanie w wierszu poleceń: trafi do { $name }, a rozmowa będzie się tu pojawiać na bieżąco.
ai-failed = AI nie odpowiedziało: { $why }
ai-stopped = zatrzymano: pytanie do { $name } porzucono i nic nie zapisano
ai-context-title = Pliki w kontekście
ai-context-all = Wszystkie
ai-context-dropped = { $count ->
        [one] jeden plik usunięto z kontekstu
        [few] { $count } pliki usunięto z kontekstu
       *[other] { $count } plików usunięto z kontekstu
    }
ai-context-hint = Enter lub Del: usuń z kontekstu — plik zostaje na dysku · F3: co jest wysyłane · Esc
ai-threads-title = Rozmowy
ai-threads-hint = Enter: kontynuuj tę rozmowę · Esc
ai-thread-resumed = kontynuacja { $name }
ai-next-estimate = następne ≈ { $tokens }
ai-next-over = następne ≈ { $tokens }, więcej, niż mieści model
ai-send-estimate = Łącznie około { $tokens } tokenów, razem z dotychczasową rozmową.
ai-send-estimate-of = Łącznie około { $tokens } tokenów, razem z dotychczasową rozmową, z { $max }, które mieści model.
ai-send-over = Łącznie około { $tokens } tokenów: więcej niż { $max }, które mieści model, więc początek może zostać ucięty. F8 usuwa plik.
ai-send-question = Pytanie trafia do { $host }.
btn-send = Wyślij

## Git (SPEC 06 §15).
menu-git = Git
git-title = Git
git-status = Co się zmieniło
git-log = Dziennik
git-blame = Blame { $name }
git-branches = Gałęzie
git-local-only = git działa na folderze na tym dysku.
git-running = Pytanie do gita…
git-failed = git: { $why }
git-clean = Nic się nie zmieniło.
git-status-title = Zmienione: w indeksie { $staged }, poza indeksem { $unstaged }
git-side-conflict = konflikt
git-side-staged = w indeksie
git-side-changed = poza indeksem
git-side-untracked = nieśledzony
git-kind-modified = zmieniony
git-kind-added = nowy
git-kind-deleted = usunięty
git-kind-renamed = zmieniona nazwa
git-kind-copied = skopiowany
git-kind-type = zmieniony typ
git-kind-conflict = do rozwiązania
git-as-tree = Pokaż zmiany jako drzewo
git-as-list = Pokaż zmiany jako listę
git-version-head = ostatni commit
git-version-index = w indeksie
git-version-disk = na dysku
git-discard-title = Odrzuć zmiany
git-discard-body = Odrzucić zmiany w { $count ->
        [one] jednym pliku
       *[other] { $count } plikach
    }?
git-discard-worktree = z powrotem do indeksu:
git-discard-both = z powrotem do ostatniego commita:
git-discard-added = z indeksu, zostaje na dysku:
git-discard-trash = do kosza:
git-discard-warning = Tego, co wraca do indeksu lub ostatniego commita, nie da się cofnąć.
git-hint = Enter: wybierz · Esc: zamknij

## The editor's character list and word completion.
charmap-title = Znaki
charmap-hint = Pisz, aby zawęzić (znak lub jego kod, U+00E9) · Enter: wstaw · Esc: wróć
editor-no-completion = Żadne inne słowo w pliku tak się nie zaczyna.

## A stored session's address, copied (NetBox's Generate URL).
net-url-copied = Skopiowano { $url }

## The system log (SPEC 06 §16).
menu-syslog = Dziennik systemowy
syslog-reading = Odczytywanie dziennika systemowego…

## Deleting lines by pattern (FAR #10).
editor-prompt-delete-matching = Usuń wiersze pasujące do (! na początku: zostaw tylko te)
editor-lines-deleted = Usunięto wierszy: { $count }

## The copy dialog's size policy (FAR #493).
copy-conflict-size = zastąp, gdy rozmiar jest inny

# Copy dialog: what an error does (FAR's issue #597)
copy-on-error = Jeśli pliku nie da się skopiować
copy-error-ask = Zapytaj
copy-error-skip = Pomiń i kontynuuj
copy-error-stop = Zatrzymaj kopiowanie

# Attributes dialog: the times copied from one file to another (FAR's issue #241)
attr-times-copied = Czasy skopiowane: Ctrl+Shift+V wkleja je w oknie innego pliku
attr-times-none = Nic jeszcze nie skopiowano: najpierw Ctrl+Shift+C w oknie pliku
attr-times-pasted = Czasy wklejone: OK je ustawi

# Editor: replacements kept by name and recent ones (FAR's issue #282)
replace-tpl-title = Zamiany
replace-tpl-hint = Enter uruchamia · Ins zachowuje ostatnią pod nazwą · Del zapomina zachowaną · Esc
replace-tpl-name = Zachowaj tę zamianę jako
replace-tpl-kept = Zachowano jako {$name}
replace-tpl-empty = Nie ma jeszcze zamian: najpierw Ctrl+F7

# Panel settings: a view and a sort per folder (FAR's issue #389)
opt-remember-view = Każdy folder ma własny widok i sortowanie

# Session editor: a cap on a copy's speed to or from the site
site-max-kbps = Limit prędkości, KB/s (puste: brak)

# Session editor: IMAP, read only
site-protocol-imap = IMAP (poczta, tylko odczyt)
site-protocol-imaps = IMAPS (poczta przez TLS, tylko odczyt)

# The AI services as a list, like the sessions
ai-services-title = Usługi AI
ai-services-hint = Enter: tę pyta ai: · Ins nowa · F4 zmień · Del usuń · Esc
ai-service-title = Usługa AI
ai-field-name = Nazwa
ai-field-api = Protokół
ai-field-url = Adres
ai-field-model = Model
ai-field-token-env = Klucz w zmiennej
ai-field-token-note = Puste: o klucz pyta się raz na sesję i nigdy się go nie zapisuje.
ai-field-system = Co słyszy na początku
ai-service-incomplete = Usługa potrzebuje nazwy i adresu
ai-service-default = ai: pyta teraz {$name}
ai-key-prompt = Klucz dla {$name} (trzymany do wyjścia z programu)
ai-api-openai = OpenAI i zgodne
ai-api-anthropic = Anthropic
ai-api-ollama = Ollama (na tym komputerze)
menu-ai = Usługi AI

# The secrets file
vault-password = Hasło pliku sekretów fcmd
vault-new = Zachowaj ten klucz w zaszyfrowanym pliku sekretów fcmd: wybierz jego hasło (puste: tylko na tę sesję)
vault-refused = Plik sekretów się nie otworzył: {$why}

# The programs from elsewhere
drive-tools = Programy zewnętrzne
drive-ai = AI
drive-git = Git
tools-found = znaleziono
tools-missing = nie zainstalowano
tools-summary = znaleziono programów zewnętrznych: { $found } z { $total }; F3 mówi, do czego służy każdy, F4 instaluje brakujący
tools-card = Program zewnętrzny
tools-card-program = Program
tools-card-where = Gdzie
tools-card-version = Wersja
tools-card-for = Do czego służy
tools-card-install = Jak zainstalować
tools-card-why = Dlaczego fcmd go uruchamia, zamiast robić to sam: F1, „Programy zewnętrzne”.
tools-install-any = zainstaluj pakiet, który dostarcza { $name }, i upewnij się, że jest w PATH
tools-already = { $name } już tu jest
tools-for-archives = archiwa: { $what }
tools-for-decompile = Shift+F6 w podglądzie na { $what }
tools-for-git = panel Git (Ctrl+Alt+G) i gałąź w znaku zachęty
tools-for-video = klatki wideo w podglądzie i szybkim podglądzie (F3, Ctrl+Q)
tools-for-open = otwieranie odnośnika w przeglądarce
tools-for-syslog = Polecenia → Dziennik systemowy
tools-for-services = panel usług
tools-card-site = Strona WWW
tools-no-package = O żadnym menedżerze pakietów na tym komputerze nie wiadomo, by miał { $name }. Otworzyć stronę pobierania?
tools-install-ask = Zainstalować { $name }, uruchamiając w terminalu:
tools-install-how = To w terminalu sudo pyta o hasło. Ctrl+Alt+E wraca do paneli, a Ctrl+R w tym panelu sprawdza ponownie.
tools-install-else = Jeśli się nie uda: { $url }
tools-install-failed = fcmd: nie zainstalowano { $name }. Pobierz z { $url }
tools-installing = instalowanie { $name } w terminalu: Ctrl+Alt+E wraca do paneli, Ctrl+R tam sprawdza ponownie
btn-install = Zainstaluj

# Plugins configuration
plugins-config-title = Wtyczki
plugins-config-hint = Enter: włącz lub wyłącz · F3 czym jest i co wnosi · Esc
plugins-broken = nie załadowała się — F3 mówi dlaczego
plugins-cannot-toggle = wtyczki, która się nie załadowała, nie można włączyć: F3 mówi, co jest z nią nie tak
plugins-on = { $id } jest włączona: jej polecenia są teraz w Ctrl+Shift+F11
plugins-on-restart = { $id } jest włączona: jej polecenia są teraz w Ctrl+Shift+F11, reszta pojawi się od następnego uruchomienia
plugins-off = { $id } jest wyłączona: jej polecenia zniknęły z Ctrl+Shift+F11
plugins-off-restart = { $id } jest wyłączona: jej polecenia zniknęły z Ctrl+Shift+F11, reszta zniknie przy następnym uruchomieniu
plugins-state-on = wł.
plugins-state-off = wył.
plugins-card-id = Identyfikator
plugins-card-name = Nazwa
plugins-card-version = Wersja
plugins-card-author = Autor
plugins-card-about = Opis
plugins-card-dir = Folder
plugins-card-state = Stan
plugins-card-brings = Wnosi
plugins-card-commands = Polecenia (Ctrl+Shift+F11):
plugins-card-notes = Komunikaty przy ładowaniu:
plugins-none = nie znaleziono wtyczek · mieszkają w { $dir }, każda we własnym folderze z plugin.toml

# The git panel
git-file-log = Dziennik { $name }
git-commit = Zatwierdź zmiany z indeksu…
git-pull = Pull
git-push = Push
git-pushing = Trwa push…
git-pulling = Trwa pull…
git-commit-prompt = Komunikat commita
git-commit-empty = Commit potrzebuje komunikatu.
git-already-on = { $branch } to gałąź, na której jesteś.
git-switch-ask = Przełączyć na { $branch }? Pliki, którymi obie gałęzie się różnią, zmienią się na dysku.
git-done = git: { $said }
git-branches-title = Gałęzie: { $count }
git-log-title = Dziennik: { $count }
git-log-of-title = Dziennik { $of }: { $count }
btn-switch = Przełącz
key-git = Git
key-git-diff = Diff
key-git-stage = Indeks
key-git-unstage = ZIndeks
key-git-discard = Odrzuć
key-git-commit = Commit
key-git-show = Pokaż
key-git-log = Dzienn
key-git-changes = Zmiany
key-git-pull = Pull
key-git-push = Push
key-git-branches = Gałęzie

undo-title = Cofnij
btn-undo = Cofnij
menu-undo = Cofnij ostatnią operację
menu-strip-meta = Usuń metadane obrazów…
undo-nothing = nie ma czego cofać: od uruchomienia programu nie było kopiowania, przenoszenia, usuwania do kosza ani nowego folderu
undo-copy =
    { $count ->
        [one] Wyślij do kosza plik utworzony przez kopiowanie:
        [few] Wyślij do kosza { $count } pliki utworzone przez kopiowanie:
       *[other] Wyślij do kosza { $count } plików utworzonych przez kopiowanie:
    }
undo-move =
    { $count ->
        [one] Przenieś plik z powrotem tam, gdzie był:
        [few] Przenieś { $count } pliki z powrotem tam, gdzie były:
       *[other] Przenieś { $count } plików z powrotem tam, gdzie były:
    }
undo-trash =
    { $count ->
        [one] Przywróć plik z kosza:
        [few] Przywróć { $count } pliki z kosza:
       *[other] Przywróć { $count } plików z kosza:
    }
undo-mkdir = Usuń nowy folder, jeśli nadal jest pusty:
strip-title = { $count ->
        [one] Metadane jednego obrazu
       *[other] Metadane { $count } obrazów
    }
strip-all = Wszystko — tekst, EXIF, XMP, IPTC, komentarze, C2PA
strip-ai = Tylko to, co zapisał model obrazów — prompt, workflow, ustawienia
strip-camera = Wszystko oprócz EXIF aparatu i praw autorskich — bez GPS
strip-hint = Enter: usuń; obraz nie jest kodowany ponownie, Ctrl+Alt+Z to cofa · Esc
strip-nothing = żaden PNG, WebP ani JPEG na tym dysku nie jest zaznaczony ani pod kursorem
strip-done = { $count ->
        [one] jeden obraz
        [few] { $count } obrazy
       *[other] { $count } obrazów
    } bez metadanych (usunięto bloków: { $blocks }), bez niczego do usunięcia: { $clean } · Ctrl+Alt+Z je przywraca
undo-restore = { $count ->
        [one] Przywróć obraz z jego metadanymi:
        [few] Przywróć { $count } obrazy z ich metadanymi:
       *[other] Przywróć { $count } obrazów z ich metadanymi:
    }
undo-done =
    { $count ->
        [one] jedna nazwa wróciła tam, gdzie była
        [few] { $count } nazwy wróciły tam, gdzie były
       *[other] { $count } nazw wróciło tam, gdzie były
    }
slow-drives = lista napędów zajęła { $secs } s
slow-drives-late = lista napędów czekała { $secs } s: { $drives } nie odpowiedziały na czas i są pokazane bez szczegółów — napęd sieciowy, którego serwera nie ma?
slow-listing = odczyt { $what } zajął { $secs } s
# editor search and undo (0.41)
btn-find-all = Wszystkie trafienia
btn-replace-all = Zamień wszystko
btn-replace-one = Zamień następny
editor-replace-title = Zamień
editor-replace-style = Zachowaj styl (foo → bar, Foo → Bar, FOO → BAR)
editor-replace-groups = W wyrażeniu regularnym $1 i { "${name}" } to zawartość grupy, $$ to dolar
editor-matches-title = Wiersze z `{ $query }`: { $count }
editor-matches-hint = Pisz, aby zawęzić · Enter: idź do wiersza · Esc: wróć


# sorting and extended attributes (0.41)
opt-sort-case = Wielkie i małe litery w sortowaniu
opt-sort-case-auto = tak jak w tym systemie
opt-sort-case-yes = rozróżniaj
opt-sort-case-no = jeden alfabet
copy-keep-xattrs = Zachowaj atrybuty rozszerzone

# editor conversions, disk diff and guides (0.41)
editor-convert-title = Zapisz plik jako
editor-convert-hint = Pisz, aby zawęzić · Enter: konwertuj (F2 zapisuje, Ctrl+Z cofa) · Esc: wróć
editor-convert-eol = Koniec wiersza: { $eol }
editor-convert-bom-on = Znacznik kolejności bajtów (BOM): zapisywany
editor-convert-bom-off = Znacznik kolejności bajtów (BOM): niezapisywany
editor-convert-page = Strona kodowa: { $page }
editor-converted-eol = { $name }: przy następnym zapisie wiersze będą się kończyć na { $what }
editor-converted-bom-on = { $name }: następny zapis doda znacznik kolejności bajtów
editor-converted-bom-off = { $name }: następny zapis nie doda znacznika kolejności bajtów
editor-converted-page = { $name }: następny zapis użyje { $what }
editor-convert-same = Plik jest już tak zapisany
editor-convert-no-bom = { $page } nie ma znacznika kolejności bajtów: mają go tylko UTF-8 i UTF-16
editor-convert-lossy = { $page } nie pomieści wszystkich znaków { $name }: nic nie zmieniono
btn-reload = Wczytaj ponownie
btn-what-changed = Co się zmieniło
editor-diff-buffer = w edytorze
editor-diff-hint = To porównanie tylko pokazuje, co się zmieniło — Esc wraca do pytania


# job journal and saved syncs (0.41)
journal-left = Kopiowań nieukończonych w poprzedniej sesji: { $count } · Alt+Shift+F6 pokazuje listę
journal-title = Nieukończone zadania
journal-hint = Enter kontynuuj · Del odrzuć · Esc zamknij
journal-none = Nie zostało żadne nieukończone zadanie
journal-row = { $kind }: { $what } → { $target } · gotowych plików: { $done }
journal-row-unreadable = nie da się odczytać: { $file } ({ $error })
journal-unreadable = { $file }: to nie jest dziennik zadania, który ta wersja umie czytać ({ $error }). Alt+Shift+F6 go pokazuje, a Del tam go usuwa
journal-resumed = Kontynuacja { $kind }: gotowych było już plików: { $done }
journal-discarded = Nieukończone zadanie odrzucono razem z tym, co zapisało w połowie
journal-cannot-resume = Tego dziennika nie da się odczytać; Del go usuwa
menu-jobs-left = Nieukończone zadania
sync-report = Zapisz raport z przebiegu do (plik)
sync-name = Zapisz tę parę jako (nazwa)
sync-saved-button = Zapisane…
sync-saved-title = Zapisane synchronizacje
sync-saved-hint = Enter uruchom · F4 zmień · Del zapomnij · Esc zamknij
sync-saved-none = Nie zapisano jeszcze żadnej synchronizacji: nadaj jej nazwę w oknie (Ctrl+Shift+Y)
sync-saved-removed = { $name } nie jest już zapisana
sync-report-local = Raport musi być plikiem na tym komputerze
sync-report-written = Raport z synchronizacji jest w { $path }
sync-report-not-written = Raportu nie udało się zapisać w { $path }: { $error }
sync-report-title = Raport z synchronizacji
sync-report-name = Nazwa: { $name }
sync-report-left = Lewy: { $path }
sync-report-right = Prawy: { $path }
sync-report-to-right = Skopiowano z lewej → na prawą: { $count }
sync-report-to-left = Skopiowano z prawej → na lewą: { $count }
sync-report-cut-right = Usunięto po prawej: { $count }
sync-report-cut-left = Usunięto po lewej: { $count }
sync-report-skipped = Zostawiono, choć się różnią: { $count }
sync-report-failed = Nie powiodło się: { $count }
sync-report-stopped = Zatrzymano w połowie, więc mogły zostać wykonane lub nie: { $count }
sync-report-not-done = Nie wykonano, bo wcześniejszy krok się nie powiódł lub został zatrzymany: { $count }
menu-sync-saved = Zapisane synchronizacje

# hex editor search
hex-search-wrapped = `{ $query }`: znaleziono po rozpoczęciu od drugiego końca pliku
hex-replace-title = Zamień
hex-replace-with = Zamień na
btn-replace = Zamień
hex-replace-ask = Zamienić { $was } w pozycji { $at } na { $with }?
hex-replace-length = zamiennik ma { $with } bajtów, a trafienie { $was }: to przesunęłoby wszystko za nim — najpierw naciśnij Ins, aby włączyć tryb wstawiania

# pixels and video (0.41)
opt-viewer-pixels = Obrazy
opt-viewer-pixels-auto = prawdziwe piksele tam, gdzie terminal je rysuje
opt-viewer-pixels-kitty = protokół graficzny kitty
opt-viewer-pixels-iterm = obrazy wbudowane iTerm2
opt-viewer-pixels-sixel = sixel
opt-viewer-pixels-off = zawsze półbloki
video-needs-program = to jest wideo ({ $what }): jego klatki rysuje `{ $program }`, który nie jest zainstalowany na tym komputerze — Shift+F11 nadal czyta kontener
video-no-frame = nie ma klatki z tego wideo: { $why }
video-playing = odtwarzanie z prędkością { $fps } klatek na sekundę, bez dźwięku — Space wstrzymuje
video-paused = wstrzymano na { $at } — Space odtwarza dalej, PgUp i PgDn skaczą o jedną dziesiątą
video-ended = wideo się skończyło — Space odtwarza je ponownie


# finding things
menu-saved-searches = Zapisane wyszukiwania
drive-searches = Zapisane wyszukiwania
find-keys = F2 zachowuje to wyszukiwanie pod nazwą · F9 zachowane
find-save-name = Zachowaj to wyszukiwanie jako
find-saved = wyszukiwanie zachowano jako { $name }
find-saved-none = nie zachowano jeszcze żadnego wyszukiwania: F2 w oknie szukania je zachowuje
find-saved-title = Zapisane wyszukiwania — Enter otwiera, F4 edytuje, Del zapomina
find-saved-running = szukanie { $name }…
find-saved-gone = wyszukiwanie { $name } zapomniano
find-loaded = wyszukiwanie { $name } wczytano
find-fuzzy = Nazwa przybliżona: litery powyżej, po kolei, najlepsze trafienie pierwsze
find-meta = Metadane (artist=bowie; camera=)
find-index = Zapytaj indeks systemu zamiast przeglądać (może pominąć nowe pliki)
tools-for-index = indeks, który Alt+F7 może zapytać zamiast przeglądać
menu-frecent = Często odwiedzane foldery
frecent-none = nie odwiedzono jeszcze żadnego folderu
frecent-gone = { $path } usunięto z często odwiedzanych folderów
frecent-hint = wpisuj litery ścieżki · Enter idź tam · Del zapomnij · Esc zamknij

# ai third round
ai-redact-off = Maskowanie jest wyłączone: wszystko jest wysyłane bez zmian
ai-redact-none = Nie znaleziono sekretów do zamaskowania
ai-redact-count = Sekretów zamaskowanych przed wysłaniem: { $count } ({ $kinds })
ai-redacted = zamaskowanych sekretów: { $count }
ai-preview-title = Co trafia do { $name } ({ $host })
ai-preview-instruction = Instrukcja
ai-preview-earlier = Dotychczasowa rozmowa
ai-preview-question = Pytanie i jego pliki
ai-preview-key = F3 pokazuje, co jest wysyłane
ai-preview-reading = Odczytywanie tego, co zostałoby wysłane…
key-ai-redact = Maskuj
ai-redact-now-on = Maskowanie jest włączone: klucze, tokeny, klucze prywatne i hasła są maskowane, zanim cokolwiek zostanie wysłane
ai-redact-now-off = Maskowanie jest wyłączone: to, co jest wysyłane, idzie bez zmian
ai-panel-redact-on = sekrety zamaskowane
ai-panel-redact-off = sekrety niezamaskowane
key-ai-local = Lokal.
ai-row-refused = nie ten komputer: odrzucono
ai-panel-local-only = tylko lokalnie
ai-local-now-on = Tryb „tylko lokalnie” jest włączony: usługa spoza tego komputera jest odrzucana i nic nie jest do niej wysyłane
ai-local-now-off = Tryb „tylko lokalnie” jest wyłączony: można pytać każdą usługę z listy
ai-local-refused = Tryb „tylko lokalnie” jest włączony: { $name } odpowiada pod { $host }, a to nie ten komputer — nic nie wysłano (F6 w panelu AI go wyłącza)
ai-rename-prompt = Czym mają się stać nazwy ({ $count })? (słowami)
ai-send-names = Instrukcja i { $count } z { $total } nazw — same nazwy, nic z plików — trafiają do { $host }.
ai-rename-ready = wzorzec modelu jest w oknie: przeczytaj podgląd, popraw to, co błędne, i dopiero OK zmienia nazwy
ai-rename-waiting = wzorzec zmiany nazw od modelu jest gotowy: Ctrl+Shift+F6 otwiera z nim okno
ai-rename-unreadable = model nie odpowiedział wzorcem zmiany nazw: jego odpowiedź jest w folderze odpowiedzi (ai:) i żadnej nazwy nie zmieniono
ai-rewrite-prompt = Co ma się stać z zaznaczonym blokiem? (słowami)
ai-rewrite-no-block = najpierw zaznacz blok (Shift+strzałki): przepisanie zastępuje dokładnie to, co zaznaczone, a blok kolumnowy nie ma kształtu, do którego można wrócić
ai-rewrite-read-only = ten plik jest tu tylko do odczytu: przepisany tekst nie miałby dokąd trafić
ai-rewrite-before = blok
ai-rewrite-after = tak, jak przepisał go model
ai-rewrite-hint = Wersja modelu obok bloku — Enter wstawia ją w miejsce bloku, Esc zostawia blok bez zmian
diff-hint-rewrite = Enter zastąp blok · Esc zostaw
ai-rewrite-applied = blok zastąpiono wersją modelu — jeden Ctrl+Z go przywraca
ai-rewrite-left = blok zostawiono bez zmian
ai-rewrite-stale = blok nie jest już taki, jak w chwili prośby o przepisanie: niczego nie zastąpiono — wersja modelu jest w folderze odpowiedzi (ai:)
ai-rewrite-same = model oddał blok bez zmian: nie ma czego zastępować
ai-answer-kept = odpowiedź to { $name } w folderze odpowiedzi (ai:) — edytor został na wierzchu
ai-failure-question = To polecenie zakończyło się kodem wyjścia { $code }: `{ $command }` — dlaczego się nie powiodło i co by to naprawiło? Krótko.
ai-send-failure = Polecenie, które się nie powiodło, i ostatnie wiersze jego wyjścia ({ $lines }) trafiają do { $host }.

# archives and jobs
arc-test-whole = { $name }: przetestowano wpisów: { $files }, { $bytes } — wszystkie całe
arc-test-bad-title = { $name }: { $bad } z { $files } wpisów okazało się uszkodzonych

# extending it
plugin-column-failed = { $column }: { $error } — jej kolumna zostaje pusta dla plików, za które nie odpowiedziała
plugins-card-columns = Kolumny (tak jak nazywa je [[column_set]]):
usermenu-far-row = Co odczytano inaczej niż w FarMenu.ini ({ $count })
usermenu-far-notes = menu pochodzi z FarMenu.ini z FAR-a: rzeczy odczytanych inaczej: { $count } — ostatni wiersz je wymienia
usermenu-far-not-run = Niczego nie uruchomiono.
find-age = Zmieniono, dni temu (7, >=30, 7-30)
find-bad-age = `{ $text }` nie jest wiekiem w dniach — { $error }
find-label-age = { $days } d
find-keys-ai = Ctrl+Alt+Q opisuje wyszukiwanie słowami: model językowy wypełnia te wiersze
ai-find-prompt = Czego szukasz? (słowami)
ai-send-words = Te słowa — i nic z paneli: żadna nazwa, ścieżka ani plik — trafiają do { $host }.
ai-find-ready = wyszukiwanie modelu jest w oknie: przeczytaj wiersze, popraw to, co błędne, i dopiero OK szuka
ai-find-waiting = wyszukiwanie modelu jest gotowe: Alt+F7 otwiera z nim okno
ai-find-unreadable = model nie odpowiedział wyszukiwaniem: jego odpowiedź jest w folderze odpowiedzi (ai:) i niczego nie szukano
ai-find-left-out = { $said } — NIE w wierszach, bo nie potrafią tego wyrazić: { $what }
# treemap and compare
usage-map-title = Użycie dysku, całe drzewo: { $path }
usage-map-walking = Mierzenie… plików: { $files }, jak dotąd { $size }
usage-map-where = { $path } · { $size } · plików: { $files }
usage-map-unreadable = nieodczytanych folderów: { $count }
usage-map-rest = mniejszych plików: { $count }
usage-map-others = innych: { $count }
usage-map-empty = Nic w tym folderze nie zajmuje miejsca
usage-map-chosen = { $name } · { $size } · { $share }%
usage-map-hint = ←↑↓→ · Enter do środka · BkSp w górę · Tab lista · G panel tam · Esc
usage-map-hint-list = ↑↓ · Enter do środka · BkSp w górę · Tab mapa · G panel tam · Esc
usage-map-hint-walking = Esc zatrzymuje
usage-map-top = To jest folder, który zmierzono: nic nie ma tu powyżej niego
usage-map-no-file = Ten wiersz to wiele małych plików, a nie jeden, do którego panel może przejść
bin-comparing = Porównywanie obu plików bajt po bajcie… Esc zatrzymuje
bin-bytes = bajtów: { $count }
bin-same = Oba pliki są takie same, bajt w bajt: po { $bytes } bajtów
bin-where = różnica { $at } z { $count } · pierwsza przy przesunięciu { $offset } · różnych bajtów: { $differing }
bin-hint = Alt+↑↓ różnice · ↑↓ PgUp PgDn Home End wiersze · Esc zamknij
diff-unified-on = Jedna kolumna: co ubyło, potem co przybyło
diff-unified-off = Dwie kolumny, obok siebie
diff-patch-title = Zapisz różnice jako łatkę (.diff)
diff-patch-saved = { $name } zapisano: jednolity diff, wierszy: { $lines }
diff-patch-nothing = Oba pliki są takie same: nie ma łatki do zapisania
# wastebasket and external editor
drive-trash = Kosz
drive-trash-closed = Kosz (ten system nie pozwala programowi go wyświetlić)
menu-trash = Kosz
menu-trash-empty = Opróżnij kosz
key-trash-restore = Przywr
key-trash-empty = Opróżn
btn-empty = Opróżnij
btn-rename = Zmień nazwę
trash-closed = kosz tego systemu można otworzyć tylko z jego własnego menedżera plików
trash-empty = kosz jest pusty
trash-failed = kosz: { $why }
trash-restored =
    { $count ->
        [one] jeden element wrócił tam, gdzie był
        [few] { $count } elementy wróciły tam, gdzie były
       *[other] { $count } elementów wróciło tam, gdzie były
    }
trash-purged =
    { $count ->
        [one] jeden element usunięto na dobre
        [few] { $count } elementy usunięto na dobre
       *[other] { $count } elementów usunięto na dobre
    }
trash-card-name = Nazwa
trash-card-from = Był w
trash-card-deleted = Usunięto
trash-card-size = Rozmiar
trash-clash-title = Przywracanie
trash-clash-body = { $path } już istnieje.
trash-clash-note = „Zastąp” wysyła istniejący do kosza; „Zmień nazwę” przywraca ten obok niego.
trash-clash-more =
    { $count ->
        [one] Jeszcze jeden element jest w tej samej sytuacji.
        [few] Jeszcze { $count } elementy są w tej samej sytuacji.
       *[other] Jeszcze { $count } elementów jest w tej samej sytuacji.
    }
trash-purge-title = Usuń na dobre
trash-purge-body =
    { $count ->
        [one] Usunąć { $name } z kosza?
        [few] Usunąć { $count } elementy z kosza?
       *[other] Usunąć { $count } elementów z kosza?
    }
trash-purge-note = To usuwa trwale: nic ich nie przywróci.
trash-empty-title = Opróżnij kosz
trash-empty-body = Usunąć wszystko, co jest w koszu?
opt-editor-external = Edytor zewnętrzny
opt-editor-use-external = Używaj edytora zewnętrznego dla F4 (Alt+F4 to wtedy wbudowany)
opt-viewer-external = Podgląd zewnętrzny
opt-viewer-use-external = Używaj podglądu zewnętrznego dla F3 (Alt+F3 to wtedy wbudowany)


# editor features
opt-editor-line-numbers = Numeruj wiersze
editor-line-numbers-on = Numery wierszy włączone
editor-line-numbers-off = Numery wierszy wyłączone
editor-prompt-pipe-block = Przepuść blok przez polecenie:
editor-prompt-pipe-file = Przepuść cały plik przez polecenie:
editor-prompt-insert-output = Wstaw to, co wypisze to polecenie:
editor-prompt-insert-file = Wstaw plik:
editor-prompt-write-block = Zapisz blok do pliku:
editor-pipe-running = Trwa { $command } …
editor-pipe-done = { $command }: zastąpiono tym, co wypisało, wierszy: { $lines } (Ctrl+Z to cofa)
editor-pipe-inserted = { $command }: wstawiono wierszy: { $lines }
editor-pipe-empty = { $command } nic nie wypisało: niczego nie wstawiono
editor-pipe-failed = { $command }: { $error } — nic nie zmieniono
editor-pipe-changed = { $command }: tekst zmienił się w trakcie działania polecenia — niczego nie zastąpiono
editor-pipe-gone = { $command }: plik, dla którego je uruchomiono, nie jest już otwarty — nic nie zmieniono
editor-pipe-column = Bloku kolumnowego nie da się przepuścić przez polecenie: zaznacz całe wiersze albo nic, by objąć cały plik
editor-file-inserted = { $name } wstawiono, wierszy: { $lines }
editor-insert-not-local = Z edytora można wczytać lub zapisać tylko plik na tym komputerze
editor-insert-too-big = za duży, by wczytać go do edytora
editor-block-written = Blok zapisano do { $name }: { $size }
editor-block-write-self = { $name } to plik właśnie edytowany: bloku nie zapisano na nim
editor-choice-hint = Pisz, aby zawęzić · Enter: wybierz · Esc: wróć
editor-syntax-title = Koloruj plik jako
editor-syntax-auto = To, co mówi jego nazwa: { $language }
editor-syntax-auto-none = To, co mówi jego nazwa: nic znanego
editor-syntax-none = Zwykły tekst, bez kolorów
editor-syntax-set = { $name } jest teraz czytany jako { $language }
editor-syntax-off = { $name } jest teraz czytany jako zwykły tekst
editor-case-heading = Zmień wielkość liter
editor-case-upper = WIELKIE LITERY
editor-case-lower = małe litery
editor-case-title = Jak W Tytule: wielka litera na początku każdego słowa
editor-case-toggle = oDWRÓĆ: wielkie stają się małymi, a małe wielkimi
editor-no-word = Nic nie jest zaznaczone, a kursor nie stoi na słowie
editor-prompt-isearch = Szukaj podczas pisania (↓ ↑ następny i poprzedni, Esc wraca)
editor-word-only = Słowo pod kursorem nie występuje nigdzie indziej
editor-not-on-word = Kursor nie stoi na słowie
editor-prompt-reflow = Na ile kolumn złożyć akapit? (sam Enter: { $width })
editor-reflowed = Akapit złożono na { $width } kolumn (Ctrl+Z to cofa)
editor-reflow-bad-width = { $width } nie jest szerokością: liczba kolumn od 8 do 1000
editor-no-paragraph = Kursor stoi na pustym wierszu: nie ma tu akapitu
editor-no-name-yet = Ten plik nie ma jeszcze nazwy: najpierw go zapisz
editor-tools-title = Polecenia edytora
editor-tool-isearch = Szukaj podczas pisania
editor-tool-word-next = Następne miejsce, w którym występuje słowo spod kursora
editor-tool-word-prev = Poprzednie miejsce, w którym występuje słowo spod kursora
editor-tool-pipe = Przepuść blok lub cały plik przez polecenie
editor-tool-write-block = Zapisz blok do pliku
editor-tool-case = Zmień wielkość liter w bloku
editor-tool-reflow = Złóż akapit ponownie na zadaną szerokość
editor-tool-insert-file = Wstaw plik w miejscu kursora
editor-tool-insert-output = Wstaw to, co wypisze polecenie
editor-tool-insert-date = Wstaw datę i godzinę
editor-tool-insert-name = Wstaw nazwę tego pliku
editor-tool-insert-path = Wstaw pełną ścieżkę tego pliku
editor-tool-line-numbers = Numery wierszy włącz lub wyłącz
editor-tool-syntax = Wybierz język, według którego plik jest kolorowany

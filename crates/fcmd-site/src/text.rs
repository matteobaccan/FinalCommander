//! The site's own words, in the seven languages the program itself speaks.
//!
//! Only the chrome and the landing page are here. The long documents — the help, the
//! changelog, the survey of the other commanders — are the repository's own files and are in
//! English; a page that shows one says so in the reader's language rather than pretending.
//! Translating those is a job for whoever wants to read them, and the offer stands.

/// One language's strings. Every field is required: a half-translated site is worse than an
/// English one, because the reader cannot tell which half they are getting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Text {
    /// The code the interface uses, and the `lang` attribute of the page.
    pub code: &'static str,
    /// What the language is called in itself, for the switcher.
    pub name: &'static str,
    pub tagline: &'static str,
    /// Three sentences: what it is, what it replaces, what it costs to try.
    pub pitch: &'static str,
    pub nav_home: &'static str,
    pub nav_features: &'static str,
    pub nav_replaces: &'static str,
    pub nav_commanders: &'static str,
    pub nav_help: &'static str,
    pub nav_changelog: &'static str,
    pub nav_download: &'static str,
    pub nav_source: &'static str,
    /// Said above a document that is in English.
    pub in_english: &'static str,
    pub download_title: &'static str,
    pub download_intro: &'static str,
    pub download_platform: &'static str,
    pub download_file: &'static str,
    pub download_checksums: &'static str,
    pub download_source: &'static str,
    pub help_title: &'static str,
    pub help_intro: &'static str,
    /// The seven chapters' names, in `HELP_PAGES` order. The chapters themselves are written
    /// in English and translated one at a time; a language that has no translation of a
    /// chapter yet still gets its own index and its own note saying what it is about to read.
    pub help_chapters: [&'static str; 7],
    pub shots_title: &'static str,
    /// The one thing the site asks of a reader: an idea, not a patch. The call to action on
    /// the landing page, and the reason the repository takes issues and not pull requests —
    /// `README.md` says the same at length.
    pub ideas: &'static str,
    /// The words on the link that opens the issue chooser, beside the sentence above.
    pub ideas_link: &'static str,
    pub footer: &'static str,
}

pub const LANGUAGES: [Text; 7] = [
    Text {
        code: "en",
        name: "English",
        tagline: "One file. Every system. Everything in it.",
        pitch: "FinalCommander is an orthodox file manager for the terminal: two panels, the \
                keys FAR Manager gave them, and no installer. One executable of about five \
                megabytes carries the archiver, the SFTP and FTP and WebDAV client, the S3 \
                browser, the viewer that opens a forty-gigabyte log at once, the hex editor \
                with its assembler, the search across files and the editor that colours code. \
                Copy it onto Linux, macOS or Windows and it is the same program with the same \
                keys; `scp` it to a server and it works there too.",
        nav_home: "Home",
        nav_features: "What works",
        nav_replaces: "What it replaces",
        nav_commanders: "The other commanders",
        nav_help: "Help",
        nav_changelog: "Changelog",
        nav_download: "Download",
        nav_source: "Source",
        in_english: "This page is one of the repository's own documents, and it is in English.",
        download_title: "Download",
        download_intro: "One file, nothing to install beside it. Unpack the archive for your \
                         machine and put `fcmd` somewhere on your `PATH`.",
        download_platform: "Platform",
        download_file: "Archive",
        download_checksums: "`SHA256SUMS` beside the archives says what each one should hash \
                             to: `sha256sum --check SHA256SUMS`.",
        download_source: "From source, with Rust 1.89 or newer:",
        help_title: "Help",
        help_intro: "The same pages `F1` shows inside the program, which are compiled into the \
                     executable and need nothing from this site.",
        help_chapters: ["Panels", "Files and archives", "Viewer", "Editor", "Servers and buckets", "Configuration", "About"],
        shots_title: "What it looks like",
        ideas: "I do not need your code. I need your ideas: open an issue with your idea and it goes into the next release.",
        ideas_link: "Open an issue",
        footer: "Drawn by the program itself, not photographed.",
    },
    Text {
        code: "it",
        name: "Italiano",
        tagline: "Un file. Ogni sistema. Tutto dentro.",
        pitch: "FinalCommander è un file manager ortodosso per il terminale: due pannelli, i \
                tasti che FAR Manager gli ha dato, nessun installatore. Un eseguibile di circa \
                cinque megabyte che contiene l'archiviatore, il client SFTP, FTP e WebDAV, il \
                browser S3, il visualizzatore che apre un log da quaranta gigabyte \
                istantaneamente, l'editor esadecimale con il suo assemblatore, la ricerca nei \
                file e l'editor che colora il codice. Copialo su Linux, macOS o Windows ed è lo \
                stesso programma con gli stessi tasti; mandalo con `scp` su un server e \
                funziona anche lì.",
        nav_home: "Home",
        nav_features: "Cosa funziona",
        nav_replaces: "Cosa sostituisce",
        nav_commanders: "Gli altri commander",
        nav_help: "Guida",
        nav_changelog: "Novità",
        nav_download: "Scarica",
        nav_source: "Codice",
        in_english: "Questa pagina è un documento del repository, ed è in inglese.",
        download_title: "Scarica",
        download_intro: "Un file, niente da installare accanto. Scompatta l'archivio della tua \
                         macchina e metti `fcmd` in una cartella del `PATH`.",
        download_platform: "Piattaforma",
        download_file: "Archivio",
        download_checksums: "`SHA256SUMS`, accanto agli archivi, dice che impronta deve avere \
                            ciascuno: `sha256sum --check SHA256SUMS`.",
        download_source: "Dai sorgenti, con Rust 1.89 o più recente:",
        help_title: "Guida",
        help_intro: "Le stesse pagine che `F1` mostra dentro il programma: sono compilate \
                     nell'eseguibile e non hanno bisogno di questo sito.",
        help_chapters: ["I pannelli", "File e archivi", "Il visualizzatore", "L'editor", "Server e bucket", "Configurazione", "Che cos'è"],
        shots_title: "Com'è fatto",
        ideas: "Non mi serve il tuo codice, mi servono le tue idee: apri una issue con la tua idea e la inserisco nella prossima release.",
        ideas_link: "Apri una issue",
        footer: "Disegnato dal programma stesso, non fotografato.",
    },
    Text {
        code: "de",
        name: "Deutsch",
        tagline: "Eine Datei. Jedes System. Alles darin.",
        pitch: "FinalCommander ist ein orthodoxer Dateimanager für das Terminal: zwei \
                Fenster, die Tasten von FAR Manager, kein Installationsprogramm. Eine \
                ausführbare Datei von etwa fünf Megabyte enthält das Archivprogramm, den \
                SFTP-, FTP- und WebDAV-Client, den S3-Browser, den Betrachter, der ein \
                vierzig Gigabyte großes Protokoll sofort öffnet, den Hex-Editor mit Assembler, \
                die Suche über Dateien hinweg und den Editor, der Code einfärbt. Kopieren Sie \
                sie auf Linux, macOS oder Windows: dasselbe Programm mit denselben Tasten — und \
                per `scp` auf einen Server kopiert läuft sie auch dort.",
        nav_home: "Start",
        nav_features: "Was läuft",
        nav_replaces: "Was sie ersetzt",
        nav_commanders: "Die anderen Commander",
        nav_help: "Hilfe",
        nav_changelog: "Änderungen",
        nav_download: "Herunterladen",
        nav_source: "Quellcode",
        in_english: "Diese Seite ist ein Dokument des Repositorys und auf Englisch.",
        download_title: "Herunterladen",
        download_intro: "Eine Datei, nichts daneben zu installieren. Archiv für Ihr System \
                         auspacken und `fcmd` in einen Ordner des `PATH` legen.",
        download_platform: "System",
        download_file: "Archiv",
        download_checksums: "`SHA256SUMS` neben den Archiven nennt die Prüfsumme jedes \
                             Archivs: `sha256sum --check SHA256SUMS`.",
        download_source: "Aus dem Quellcode, mit Rust 1.89 oder neuer:",
        help_title: "Hilfe",
        help_intro: "Dieselben Seiten, die `F1` im Programm zeigt: Sie sind in die ausführbare \
                     Datei einkompiliert und brauchen diese Website nicht.",
        help_chapters: ["Die Panels", "Dateien und Archive", "Der Betrachter", "Der Editor", "Server und Buckets", "Konfiguration", "Worum es geht"],
        shots_title: "Wie es aussieht",
        ideas: "Ich brauche nicht deinen Code, ich brauche deine Ideen: öffne ein Issue mit deiner Idee, und sie kommt in die nächste Version.",
        ideas_link: "Ein Issue öffnen",
        footer: "Vom Programm selbst gezeichnet, nicht abfotografiert.",
    },
    Text {
        code: "es",
        name: "Español",
        tagline: "Un archivo. Cualquier sistema. Todo dentro.",
        pitch: "FinalCommander es un gestor de archivos ortodoxo para la terminal: dos \
                paneles, las teclas que les dio FAR Manager y ningún instalador. Un ejecutable \
                de unos cinco megabytes que lleva dentro el archivador, el cliente SFTP, FTP y \
                WebDAV, el navegador de S3, el visor que abre un registro de cuarenta \
                gigabytes al instante, el editor hexadecimal con su ensamblador, la búsqueda \
                entre archivos y el editor que colorea el código. Cópialo en Linux, macOS o \
                Windows y es el mismo programa con las mismas teclas; mándalo con `scp` a un \
                servidor y también funciona allí.",
        nav_home: "Inicio",
        nav_features: "Qué funciona",
        nav_replaces: "Qué sustituye",
        nav_commanders: "Los otros commander",
        nav_help: "Ayuda",
        nav_changelog: "Cambios",
        nav_download: "Descargar",
        nav_source: "Código",
        in_english: "Esta página es un documento del repositorio y está en inglés.",
        download_title: "Descargar",
        download_intro: "Un archivo, nada que instalar al lado. Descomprime el archivo de tu \
                         máquina y pon `fcmd` en una carpeta del `PATH`.",
        download_platform: "Plataforma",
        download_file: "Archivo",
        download_checksums: "`SHA256SUMS`, junto a los archivos, dice qué huella debe tener \
                             cada uno: `sha256sum --check SHA256SUMS`.",
        download_source: "Desde el código, con Rust 1.89 o más reciente:",
        help_title: "Ayuda",
        help_intro: "Las mismas páginas que `F1` muestra dentro del programa: están compiladas \
                     en el ejecutable y no necesitan este sitio.",
        help_chapters: ["Los paneles", "Archivos y comprimidos", "El visor", "El editor", "Servidores y buckets", "Configuración", "Qué es esto"],
        shots_title: "Qué aspecto tiene",
        ideas: "No necesito tu código, necesito tus ideas: abre una issue con tu idea y entrará en la próxima versión.",
        ideas_link: "Abrir una issue",
        footer: "Dibujado por el propio programa, no fotografiado.",
    },
    Text {
        code: "fr",
        name: "Français",
        tagline: "Un fichier. Tous les systèmes. Tout dedans.",
        pitch: "FinalCommander est un gestionnaire de fichiers orthodoxe pour le terminal : \
                deux panneaux, les touches que FAR Manager leur a données, aucun installateur. \
                Un exécutable d'environ cinq mégaoctets qui contient l'archiveur, le client \
                SFTP, FTP et WebDAV, le navigateur S3, la visionneuse qui ouvre un journal de \
                quarante gigaoctets d'un coup, l'éditeur hexadécimal et son assembleur, la \
                recherche à travers les fichiers et l'éditeur qui colore le code. Copiez-le sur \
                Linux, macOS ou Windows : c'est le même programme avec les mêmes touches — et \
                envoyé par `scp` sur un serveur, il y marche aussi.",
        nav_home: "Accueil",
        nav_features: "Ce qui marche",
        nav_replaces: "Ce qu'il remplace",
        nav_commanders: "Les autres commander",
        nav_help: "Aide",
        nav_changelog: "Nouveautés",
        nav_download: "Télécharger",
        nav_source: "Sources",
        in_english: "Cette page est un document du dépôt, et elle est en anglais.",
        download_title: "Télécharger",
        download_intro: "Un fichier, rien à installer à côté. Décompressez l'archive de votre \
                         machine et mettez `fcmd` dans un dossier du `PATH`.",
        download_platform: "Plateforme",
        download_file: "Archive",
        download_checksums: "`SHA256SUMS`, à côté des archives, donne l'empreinte de chacune : \
                             `sha256sum --check SHA256SUMS`.",
        download_source: "Depuis les sources, avec Rust 1.89 ou plus récent :",
        help_title: "Aide",
        help_intro: "Les mêmes pages que `F1` affiche dans le programme : elles sont compilées \
                     dans l'exécutable et n'ont pas besoin de ce site.",
        help_chapters: ["Les panneaux", "Fichiers et archives", "La visionneuse", "L'éditeur", "Serveurs et buckets", "Configuration", "Ce que c'est"],
        shots_title: "À quoi cela ressemble",
        ideas: "Je n'ai pas besoin de votre code, j'ai besoin de vos idées : ouvrez une issue avec votre idée et elle entrera dans la prochaine version.",
        ideas_link: "Ouvrir une issue",
        footer: "Dessiné par le programme lui-même, pas photographié.",
    },
    Text {
        code: "ru",
        name: "Русский",
        tagline: "Один файл. Любая система. Всё внутри.",
        pitch: "FinalCommander — ортодоксальный файловый менеджер для терминала: две панели, \
                клавиши FAR Manager и никакого установщика. Один исполняемый файл размером \
                около пяти мегабайт содержит архиватор, клиент SFTP, FTP и WebDAV, браузер S3, \
                просмотрщик, который открывает журнал на сорок гигабайт мгновенно, \
                шестнадцатеричный редактор с ассемблером, поиск по файлам и редактор с \
                подсветкой кода. Скопируйте его на Linux, macOS или Windows — это та же \
                программа с теми же клавишами; отправьте через `scp` на сервер, и там он тоже \
                работает.",
        nav_home: "Главная",
        nav_features: "Что работает",
        nav_replaces: "Что он заменяет",
        nav_commanders: "Другие командеры",
        nav_help: "Справка",
        nav_changelog: "Изменения",
        nav_download: "Скачать",
        nav_source: "Исходники",
        in_english: "Эта страница — документ из репозитория, и она на английском.",
        download_title: "Скачать",
        download_intro: "Один файл, рядом ставить нечего. Распакуйте архив для своей системы и \
                         положите `fcmd` в каталог из `PATH`.",
        download_platform: "Система",
        download_file: "Архив",
        download_checksums: "`SHA256SUMS` рядом с архивами говорит, какой у каждого хеш: \
                             `sha256sum --check SHA256SUMS`.",
        download_source: "Из исходников, с Rust 1.89 или новее:",
        help_title: "Справка",
        help_intro: "Те же страницы, что `F1` показывает в программе: они вкомпилированы в \
                     исполняемый файл и этот сайт им не нужен.",
        help_chapters: ["Панели", "Файлы и архивы", "Просмотрщик", "Редактор", "Серверы и бакеты", "Настройки", "Что это такое"],
        shots_title: "Как это выглядит",
        ideas: "Мне нужен не ваш код, а ваши идеи: откройте issue со своей идеей, и она войдёт в следующий выпуск.",
        ideas_link: "Открыть issue",
        footer: "Нарисовано самой программой, а не снято с экрана.",
    },
    Text {
        code: "zh",
        name: "中文",
        tagline: "一个文件。任何系统。应有尽有。",
        pitch: "FinalCommander 是终端里的正统文件管理器：两个面板、FAR Manager 的按键，不需要安装程序。\
                一个约五兆字节的可执行文件里装着压缩工具、SFTP/FTP/WebDAV 客户端、S3 浏览器、能瞬间打开四十 GB \
                日志的查看器、带汇编器的十六进制编辑器、跨文件搜索，以及会着色的代码编辑器。把它复制到 Linux、\
                macOS 或 Windows 上，都是同一个程序、同一套按键；用 `scp` 送到服务器上，在那里同样能用。",
        nav_home: "首页",
        nav_features: "已实现",
        nav_replaces: "它替代了什么",
        nav_commanders: "其他 commander",
        nav_help: "帮助",
        nav_changelog: "更新记录",
        nav_download: "下载",
        nav_source: "源码",
        in_english: "本页是仓库里的文档，内容为英文。",
        download_title: "下载",
        download_intro: "一个文件，旁边无需安装任何东西。解压适合你机器的压缩包，把 `fcmd` 放进 `PATH` 里。",
        download_platform: "平台",
        download_file: "压缩包",
        download_checksums: "压缩包旁边的 `SHA256SUMS` 给出了每个文件的校验和：`sha256sum --check SHA256SUMS`。",
        download_source: "从源码构建，需要 Rust 1.89 或更新：",
        help_title: "帮助",
        help_intro: "与程序里按 `F1` 看到的是同一批页面：它们编译进了可执行文件，并不依赖这个网站。",
        help_chapters: ["两个面板", "文件与压缩包", "查看器", "编辑器", "服务器与存储桶", "配置", "这是什么"],
        shots_title: "界面是什么样",
        ideas: "我需要的不是你的代码，而是你的想法：带着想法开一个 issue，它会进入下一个版本。",
        ideas_link: "开一个 issue",
        footer: "由程序自己绘制，而非截图。",
    },
];

/// The English strings, which every page falls back to for anything not translated.
pub fn english() -> &'static Text {
    &LANGUAGES[0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn there_is_one_entry_per_interface_language_and_none_is_empty() {
        // The same seven the program itself speaks, in the same order as its catalogues.
        let codes: Vec<&str> = LANGUAGES.iter().map(|t| t.code).collect();
        assert_eq!(codes, ["en", "it", "de", "es", "fr", "ru", "zh"]);
        assert_eq!(english().code, "en");
        assert_eq!(LANGUAGES[1].name, "Italiano");

        for text in LANGUAGES {
            // A half-translated site is worse than an English one: the reader cannot tell
            // which half they are getting. So every field of every language is filled in.
            for (field, value) in [
                ("name", text.name),
                ("tagline", text.tagline),
                ("pitch", text.pitch),
                ("nav_home", text.nav_home),
                ("nav_features", text.nav_features),
                ("nav_replaces", text.nav_replaces),
                ("nav_commanders", text.nav_commanders),
                ("nav_help", text.nav_help),
                ("nav_changelog", text.nav_changelog),
                ("nav_download", text.nav_download),
                ("nav_source", text.nav_source),
                ("in_english", text.in_english),
                ("download_title", text.download_title),
                ("download_intro", text.download_intro),
                ("download_platform", text.download_platform),
                ("download_file", text.download_file),
                ("download_checksums", text.download_checksums),
                ("download_source", text.download_source),
                ("help_title", text.help_title),
                ("help_intro", text.help_intro),
                ("shots_title", text.shots_title),
                ("ideas", text.ideas),
                ("ideas_link", text.ideas_link),
                ("footer", text.footer),
            ] {
                assert!(!value.trim().is_empty(), "{}: {field} is empty", text.code);
            }
        }
    }

    #[test]
    fn the_pitch_says_what_it_is_in_every_language() {
        // Not a translation check — nobody can test that here — but the one thing that would
        // show a copy-and-paste slip: every pitch names the program's own promise, which is
        // the same word in all seven.
        for text in LANGUAGES {
            assert!(
                text.pitch.contains("FinalCommander"),
                "{}: the pitch does not name the program",
                text.code
            );
            assert!(text.pitch.len() > 200, "{}: the pitch is a stub", text.code);
        }
    }
}

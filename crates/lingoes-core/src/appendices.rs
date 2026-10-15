// Appendices data and tools inspired by Lingoes 2.9.2 official tools.
// Clean-room structured data for Irregular Verbs, Unit Conversion, IPA Chart, and Dialing Codes.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrregularVerb {
    pub v1: &'static str,
    pub v2: &'static str,
    pub v3: &'static str,
    pub meaning_vi: &'static str,
}

pub static IRREGULAR_VERBS: &[IrregularVerb] = &[
    IrregularVerb {
        v1: "abide",
        v2: "abode, abided",
        v3: "abode, abided",
        meaning_vi: "tuân theo, chịu đựng",
    },
    IrregularVerb {
        v1: "alight",
        v2: "alighted, alit",
        v3: "alighted, alit",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "arise",
        v2: "arose",
        v3: "arisen",
        meaning_vi: "xuất hiện, nổi lên",
    },
    IrregularVerb {
        v1: "awake",
        v2: "awoke",
        v3: "awoke, awaked",
        meaning_vi: "thức giấc, đánh thức",
    },
    IrregularVerb {
        v1: "be",
        v2: "was, were",
        v3: "been",
        meaning_vi: "thì, là, ở",
    },
    IrregularVerb {
        v1: "bear",
        v2: "bore",
        v3: "borne, born",
        meaning_vi: "chịu đựng, sinh nở",
    },
    IrregularVerb {
        v1: "beat",
        v2: "beat",
        v3: "beaten",
        meaning_vi: "đánh, đập",
    },
    IrregularVerb {
        v1: "become",
        v2: "became",
        v3: "become",
        meaning_vi: "trở thành, trở nên",
    },
    IrregularVerb {
        v1: "befall",
        v2: "befell",
        v3: "befallen",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "beget",
        v2: "begot",
        v3: "begotten, begot",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "begin",
        v2: "began",
        v3: "begun",
        meaning_vi: "bắt đầu",
    },
    IrregularVerb {
        v1: "behold",
        v2: "beheld",
        v3: "beheld",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "bend",
        v2: "bent",
        v3: "bent",
        meaning_vi: "bẻ cong, cúi",
    },
    IrregularVerb {
        v1: "bereave",
        v2: "bereaved, bereft",
        v3: "bereaved, bereft",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "beseech",
        v2: "besought, beseeched",
        v3: "besought, beseeched",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "beset",
        v2: "beset",
        v3: "beset",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "bespeak",
        v2: "bespoke",
        v3: "bespoken, bespoke",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "bespread",
        v2: "bespread",
        v3: "bespread",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "bestrew",
        v2: "bestrewed",
        v3: "bestrewed, bestrewn",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "bestride",
        v2: "bestrode",
        v3: "bestridden, bestrid, bestrode",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "bet",
        v2: "bet, betted",
        v3: "bet, betted",
        meaning_vi: "đánh cược",
    },
    IrregularVerb {
        v1: "betake",
        v2: "betook",
        v3: "betaken",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "bethink",
        v2: "bethought",
        v3: "bethought",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "bid",
        v2: "bade, bid",
        v3: "bidden, bid",
        meaning_vi: "trả giá, đấu thầu",
    },
    IrregularVerb {
        v1: "bide",
        v2: "bode, bided",
        v3: "bided",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "bind",
        v2: "bound",
        v3: "bound",
        meaning_vi: "trói buộc, buộc lại",
    },
    IrregularVerb {
        v1: "bite",
        v2: "bit",
        v3: "bitten, bit",
        meaning_vi: "cắn, ngoạm",
    },
    IrregularVerb {
        v1: "bleed",
        v2: "bled",
        v3: "bled",
        meaning_vi: "chảy máu",
    },
    IrregularVerb {
        v1: "blend",
        v2: "blended, blent",
        v3: "blended, blent",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "bless",
        v2: "blessed, blest",
        v3: "blessed, blest",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "blow",
        v2: "blew",
        v3: "blown",
        meaning_vi: "thổi gió",
    },
    IrregularVerb {
        v1: "break",
        v2: "broke",
        v3: "broken",
        meaning_vi: "làm vỡ, bẻ gãy",
    },
    IrregularVerb {
        v1: "breed",
        v2: "bred",
        v3: "bred",
        meaning_vi: "nuôi dưỡng, lai tạo",
    },
    IrregularVerb {
        v1: "bring",
        v2: "brought",
        v3: "brought",
        meaning_vi: "mang lại, đem đến",
    },
    IrregularVerb {
        v1: "broadcast",
        v2: "broadcast, broadcasted",
        v3: "broadcast, broadcasted",
        meaning_vi: "phát thanh, truyền hình",
    },
    IrregularVerb {
        v1: "browbeat",
        v2: "browbeat",
        v3: "browbeaten",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "build",
        v2: "built",
        v3: "built",
        meaning_vi: "xây dựng",
    },
    IrregularVerb {
        v1: "burn",
        v2: "burnt, burned",
        v3: "burnt, burned",
        meaning_vi: "đốt cháy, thiêu",
    },
    IrregularVerb {
        v1: "burst",
        v2: "burst",
        v3: "burst",
        meaning_vi: "nổ tung, vỡ tung",
    },
    IrregularVerb {
        v1: "buy",
        v2: "bought",
        v3: "bought",
        meaning_vi: "mua sắm",
    },
    IrregularVerb {
        v1: "can",
        v2: "could",
        v3: "-",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "cast",
        v2: "cast",
        v3: "cast",
        meaning_vi: "quăng, thả, đúc",
    },
    IrregularVerb {
        v1: "catch",
        v2: "caught",
        v3: "caught",
        meaning_vi: "bắt lấy, đón lấy",
    },
    IrregularVerb {
        v1: "chide",
        v2: "chid, chided",
        v3: "chid, chidden, chided",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "choose",
        v2: "chose",
        v3: "chosen",
        meaning_vi: "chọn lựa",
    },
    IrregularVerb {
        v1: "cleave",
        v2: "cleaved, cleft, clove",
        v3: "cleaved, cleft",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "cling",
        v2: "clung",
        v3: "clung",
        meaning_vi: "bám víu",
    },
    IrregularVerb {
        v1: "clothe",
        v2: "clothed, clad",
        v3: "clothed, clad",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "come",
        v2: "came",
        v3: "come",
        meaning_vi: "đi đến, tới",
    },
    IrregularVerb {
        v1: "cost",
        v2: "cost",
        v3: "cost",
        meaning_vi: "trị giá, tốn kém",
    },
    IrregularVerb {
        v1: "creep",
        v2: "crept",
        v3: "crept",
        meaning_vi: "bò, trườn",
    },
    IrregularVerb {
        v1: "crow",
        v2: "crowed, crew",
        v3: "crowed",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "cut",
        v2: "cut",
        v3: "cut",
        meaning_vi: "cắt gọt",
    },
    IrregularVerb {
        v1: "dare",
        v2: "dared, durst",
        v3: "dared",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "deal",
        v2: "dealt",
        v3: "dealt",
        meaning_vi: "giao dịch, giải quyết",
    },
    IrregularVerb {
        v1: "dig",
        v2: "dug",
        v3: "dug",
        meaning_vi: "đào bới",
    },
    IrregularVerb {
        v1: "dispread",
        v2: "dispread",
        v3: "dispread",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "do",
        v2: "did",
        v3: "done",
        meaning_vi: "làm, thực hiện",
    },
    IrregularVerb {
        v1: "draw",
        v2: "drew",
        v3: "drawn",
        meaning_vi: "vẽ, kéo",
    },
    IrregularVerb {
        v1: "dream",
        v2: "dreamed, dreamt",
        v3: "dreamed, dreamt",
        meaning_vi: "mơ mộng",
    },
    IrregularVerb {
        v1: "drink",
        v2: "drank",
        v3: "drunk",
        meaning_vi: "uống nước",
    },
    IrregularVerb {
        v1: "drive",
        v2: "drove",
        v3: "driven",
        meaning_vi: "lái xe",
    },
    IrregularVerb {
        v1: "dwell",
        v2: "dwelt, dwelled",
        v3: "dwelt, dwelled",
        meaning_vi: "trú ngụ, ở",
    },
    IrregularVerb {
        v1: "eat",
        v2: "ate",
        v3: "eaten",
        meaning_vi: "ăn uống",
    },
    IrregularVerb {
        v1: "fall",
        v2: "fell",
        v3: "fallen",
        meaning_vi: "rơi, ngã",
    },
    IrregularVerb {
        v1: "feed",
        v2: "fed",
        v3: "fed",
        meaning_vi: "cho ăn",
    },
    IrregularVerb {
        v1: "feel",
        v2: "felt",
        v3: "felt",
        meaning_vi: "cảm thấy",
    },
    IrregularVerb {
        v1: "fight",
        v2: "fought",
        v3: "fought",
        meaning_vi: "chiến đấu, đánh",
    },
    IrregularVerb {
        v1: "find",
        v2: "found",
        v3: "found",
        meaning_vi: "tìm thấy",
    },
    IrregularVerb {
        v1: "flee",
        v2: "fled",
        v3: "fled",
        meaning_vi: "chạy trốn",
    },
    IrregularVerb {
        v1: "fling",
        v2: "flung",
        v3: "flung",
        meaning_vi: "quăng mạnh",
    },
    IrregularVerb {
        v1: "fly",
        v2: "flew",
        v3: "flown",
        meaning_vi: "bay",
    },
    IrregularVerb {
        v1: "forbear",
        v2: "forbore",
        v3: "forborne",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "forbid",
        v2: "forbade, forbad",
        v3: "forbidden",
        meaning_vi: "ngăn cấm",
    },
    IrregularVerb {
        v1: "forecast",
        v2: "forecast, forecasted",
        v3: "forecast, forecasted",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "fordo",
        v2: "fordid",
        v3: "fordone",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "forego",
        v2: "forewent",
        v3: "foregone",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "foreknow",
        v2: "foreknew",
        v3: "foreknown",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "forerun",
        v2: "foreran",
        v3: "forerun",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "foresee",
        v2: "foresaw",
        v3: "foreseen",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "foreshow",
        v2: "foreshowed",
        v3: "foreshown",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "foretell",
        v2: "foretold",
        v3: "foretold",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "forget",
        v2: "forgot",
        v3: "forgotten, forgot",
        meaning_vi: "quên lãng",
    },
    IrregularVerb {
        v1: "forgive",
        v2: "forgave",
        v3: "forgiven",
        meaning_vi: "tha thứ",
    },
    IrregularVerb {
        v1: "forsake",
        v2: "forsook",
        v3: "forsaken",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "forswear",
        v2: "forswore",
        v3: "forsworn",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "freeze",
        v2: "froze",
        v3: "frozen",
        meaning_vi: "đóng băng, lạnh cóng",
    },
    IrregularVerb {
        v1: "gainsay",
        v2: "gainsaid",
        v3: "gainsaid",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "get",
        v2: "got",
        v3: "got, gotten",
        meaning_vi: "có được, nhận được",
    },
    IrregularVerb {
        v1: "gild",
        v2: "gilded, gilt",
        v3: "gilded",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "gird",
        v2: "girded, girt",
        v3: "girded, girt",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "give",
        v2: "gave",
        v3: "given",
        meaning_vi: "cho, tặng",
    },
    IrregularVerb {
        v1: "go",
        v2: "went",
        v3: "gone",
        meaning_vi: "đi lại",
    },
    IrregularVerb {
        v1: "grave",
        v2: "graved",
        v3: "graven, graved",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "grind",
        v2: "ground",
        v3: "ground",
        meaning_vi: "nghiền, xay",
    },
    IrregularVerb {
        v1: "grow",
        v2: "grew",
        v3: "grown",
        meaning_vi: "lớn lên, phát triển",
    },
    IrregularVerb {
        v1: "hamstring",
        v2: "hamstringed, hamstrung",
        v3: "hamstringed, hamstrung",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "hang",
        v2: "hung",
        v3: "hung",
        meaning_vi: "treo, mắc",
    },
    IrregularVerb {
        v1: "have",
        v2: "had",
        v3: "had",
        meaning_vi: "có, sở hữu",
    },
    IrregularVerb {
        v1: "hear",
        v2: "heard",
        v3: "heard",
        meaning_vi: "nghe thấy",
    },
    IrregularVerb {
        v1: "heave",
        v2: "heaved, hove",
        v3: "heaved, hove",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "hew",
        v2: "hewed",
        v3: "hewed, hewn",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "hide",
        v2: "hid",
        v3: "hidden, hid",
        meaning_vi: "trốn, giấu",
    },
    IrregularVerb {
        v1: "hit",
        v2: "hit",
        v3: "hit",
        meaning_vi: "đánh trúng",
    },
    IrregularVerb {
        v1: "hold",
        v2: "held",
        v3: "held",
        meaning_vi: "cầm giữ, tổ chức",
    },
    IrregularVerb {
        v1: "hurt",
        v2: "hurt",
        v3: "hurt",
        meaning_vi: "làm đau, tổn thương",
    },
    IrregularVerb {
        v1: "inlay",
        v2: "inlaid",
        v3: "inlaid",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "keep",
        v2: "kept",
        v3: "kept",
        meaning_vi: "giữ, duy trì",
    },
    IrregularVerb {
        v1: "kneel",
        v2: "knelt, kneeled",
        v3: "knelt, kneeled",
        meaning_vi: "quỳ gối",
    },
    IrregularVerb {
        v1: "knit",
        v2: "knitted, knit",
        v3: "knitted, knit",
        meaning_vi: "đan len",
    },
    IrregularVerb {
        v1: "know",
        v2: "knew",
        v3: "known",
        meaning_vi: "biết, hiểu",
    },
    IrregularVerb {
        v1: "lade",
        v2: "laded",
        v3: "laded, laden",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "lay",
        v2: "laid",
        v3: "laid",
        meaning_vi: "đặt, để, đẻ trứng",
    },
    IrregularVerb {
        v1: "lead",
        v2: "led",
        v3: "led",
        meaning_vi: "dẫn dắt, lãnh đạo",
    },
    IrregularVerb {
        v1: "lean",
        v2: "leaned, leant",
        v3: "leaned, leant",
        meaning_vi: "dựa, tựa",
    },
    IrregularVerb {
        v1: "leap",
        v2: "leapt, leaped",
        v3: "leapt, leaped",
        meaning_vi: "nhảy vọt",
    },
    IrregularVerb {
        v1: "learn",
        v2: "learnt, learned",
        v3: "learnt, learned",
        meaning_vi: "học tập",
    },
    IrregularVerb {
        v1: "leave",
        v2: "left",
        v3: "left",
        meaning_vi: "rời đi, để lại",
    },
    IrregularVerb {
        v1: "lend",
        v2: "lent",
        v3: "lent",
        meaning_vi: "cho vay, cho mượn",
    },
    IrregularVerb {
        v1: "let",
        v2: "let",
        v3: "let",
        meaning_vi: "cho phép",
    },
    IrregularVerb {
        v1: "lie",
        v2: "lay",
        v3: "lain",
        meaning_vi: "nằm (nghỉ)",
    },
    IrregularVerb {
        v1: "light",
        v2: "lit",
        v3: "lit",
        meaning_vi: "thắp sáng, soi sáng",
    },
    IrregularVerb {
        v1: "lose",
        v2: "lost",
        v3: "lost",
        meaning_vi: "đánh mất, thua",
    },
    IrregularVerb {
        v1: "make",
        v2: "made",
        v3: "made",
        meaning_vi: "chế tạo, tạo ra",
    },
    IrregularVerb {
        v1: "may",
        v2: "might",
        v3: "-",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "mean",
        v2: "meant",
        v3: "meant",
        meaning_vi: "có ý nghĩa là",
    },
    IrregularVerb {
        v1: "meet",
        v2: "met",
        v3: "met",
        meaning_vi: "gặp gỡ",
    },
    IrregularVerb {
        v1: "melt",
        v2: "melted",
        v3: "melted, melten",
        meaning_vi: "tan chảy",
    },
    IrregularVerb {
        v1: "misdeal",
        v2: "misdealt",
        v3: "misdealt",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "misgive",
        v2: "misgave",
        v3: "misgiven",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "mislay",
        v2: "mislaid",
        v3: "mislaid",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "mislead",
        v2: "misled",
        v3: "misled",
        meaning_vi: "làm lạc lối",
    },
    IrregularVerb {
        v1: "mistake",
        v2: "mistook",
        v3: "mistaken",
        meaning_vi: "phạm sai lầm",
    },
    IrregularVerb {
        v1: "misunderstand",
        v2: "misunderstood",
        v3: "misunderstood",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "mow",
        v2: "mowed",
        v3: "mowed, mown",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "must",
        v2: "must",
        v3: "-",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "ought",
        v2: "ought",
        v3: "-",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "outbid",
        v2: "outbade, outbid",
        v3: "outbidden, outbid",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "outbreed",
        v2: "outbred",
        v3: "outbred",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "outdo",
        v2: "outdid",
        v3: "outdone",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "outeat",
        v2: "outate",
        v3: "outeaten",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "outfight",
        v2: "outfought",
        v3: "outfought",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "outgo",
        v2: "outwent",
        v3: "outgone",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "outgrow",
        v2: "outgrew",
        v3: "outgrown",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "outlay",
        v2: "outlaid",
        v3: "outlaid",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "outride",
        v2: "outrode",
        v3: "outridden",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "outrun",
        v2: "outran",
        v3: "outrun",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "outsell",
        v2: "outsold",
        v3: "outsold",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "outshine",
        v2: "outshone",
        v3: "outshone",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "outshoot",
        v2: "outshot",
        v3: "outshot",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "outsit",
        v2: "outsat",
        v3: "outsat",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "outspend",
        v2: "outspent",
        v3: "outspent",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "outspread",
        v2: "outspread",
        v3: "outspread",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "outthrow",
        v2: "outthrew",
        v3: "outthrown",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "outthrust",
        v2: "outthrust",
        v3: "outthrust",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "outwear",
        v2: "outwore",
        v3: "outworn",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "overbear",
        v2: "overbore",
        v3: "overborne",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "overbid",
        v2: "overbid",
        v3: "overbidden, overbid",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "overblow",
        v2: "overblew",
        v3: "overblown",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "overbuild",
        v2: "overbuilt",
        v3: "overbuilt",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "overbuy",
        v2: "overbought",
        v3: "overbought",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "overcast",
        v2: "overcast",
        v3: "overcast",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "overcome",
        v2: "overcame",
        v3: "overcome",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "overdo",
        v2: "overdid",
        v3: "overdone",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "overdraw",
        v2: "overdrew",
        v3: "overdrawn",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "overdrive",
        v2: "overdrove",
        v3: "overdriven",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "overeat",
        v2: "overate",
        v3: "overeaten",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "overfeed",
        v2: "overfed",
        v3: "overfed",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "overfly",
        v2: "overflew",
        v3: "overflown",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "overgrow",
        v2: "overgrew",
        v3: "overgrown",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "overhang",
        v2: "overhung",
        v3: "overhung",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "overhear",
        v2: "overheard",
        v3: "overheard",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "overlade",
        v2: "overladed",
        v3: "overladed, overladen",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "overlay",
        v2: "overlaid",
        v3: "overlaid",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "overleap",
        v2: "overleapt, overleaped",
        v3: "overleapt, overleaped",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "overlie",
        v2: "overlay",
        v3: "overlain",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "overpay",
        v2: "overpaid",
        v3: "overpaid",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "override",
        v2: "overrode",
        v3: "overridden",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "overrun",
        v2: "overran",
        v3: "overrun",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "oversee",
        v2: "oversaw",
        v3: "overseen",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "oversell",
        v2: "oversold",
        v3: "oversold",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "overset",
        v2: "overset",
        v3: "overset",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "oversew",
        v2: "oversewed",
        v3: "oversewed, oversewn",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "overshoot",
        v2: "overshot",
        v3: "overshot",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "oversleep",
        v2: "overslept",
        v3: "overslept",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "overspend",
        v2: "overspent",
        v3: "overspent",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "overspread",
        v2: "overspread",
        v3: "overspread",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "overtake",
        v2: "overtook",
        v3: "overtaken",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "overthrow",
        v2: "overthrew",
        v3: "overthrown",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "overwind",
        v2: "overwound",
        v3: "overwound",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "overwrite",
        v2: "overwrote",
        v3: "overwritten",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "partake",
        v2: "partook",
        v3: "partaken",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "pay",
        v2: "paid",
        v3: "paid",
        meaning_vi: "chi trả, trả tiền",
    },
    IrregularVerb {
        v1: "precast",
        v2: "precast",
        v3: "precast",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "prechoose",
        v2: "prechose",
        v3: "prechosen",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "prove",
        v2: "proved",
        v3: "proved, proven",
        meaning_vi: "chứng minh",
    },
    IrregularVerb {
        v1: "put",
        v2: "put",
        v3: "put",
        meaning_vi: "đặt, để vào",
    },
    IrregularVerb {
        v1: "quit",
        v2: "quitted, quit",
        v3: "quitted, quit",
        meaning_vi: "từ bỏ, nghỉ",
    },
    IrregularVerb {
        v1: "read",
        v2: "read",
        v3: "read",
        meaning_vi: "đọc sách",
    },
    IrregularVerb {
        v1: "reave",
        v2: "reaved, reft",
        v3: "reaved, reft",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "rebuild",
        v2: "rebuilt",
        v3: "rebuilt",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "recast",
        v2: "recast",
        v3: "recast",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "reeve",
        v2: "rove, reeved",
        v3: "rove, reeved",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "relay",
        v2: "relaid",
        v3: "relaid",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "rend",
        v2: "rent",
        v3: "rent",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "repay",
        v2: "repaid",
        v3: "repaid",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "reset",
        v2: "reset",
        v3: "reset",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "retell",
        v2: "retold",
        v3: "retold",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "rid",
        v2: "rid, ridded",
        v3: "rid, ridded",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "ride",
        v2: "rode",
        v3: "ridden",
        meaning_vi: "cưỡi, lái",
    },
    IrregularVerb {
        v1: "ring",
        v2: "rang",
        v3: "rung",
        meaning_vi: "rung chuông",
    },
    IrregularVerb {
        v1: "rise",
        v2: "rose",
        v3: "risen",
        meaning_vi: "dâng lên, mọc",
    },
    IrregularVerb {
        v1: "rive",
        v2: "riven",
        v3: "riven, rived",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "run",
        v2: "ran",
        v3: "run",
        meaning_vi: "chạy bộ",
    },
    IrregularVerb {
        v1: "saw",
        v2: "sawed",
        v3: "sawn, sawed",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "say",
        v2: "said",
        v3: "said",
        meaning_vi: "nói rằng",
    },
    IrregularVerb {
        v1: "see",
        v2: "saw",
        v3: "seen",
        meaning_vi: "nhìn thấy",
    },
    IrregularVerb {
        v1: "seek",
        v2: "sought",
        v3: "sought",
        meaning_vi: "tìm kiếm",
    },
    IrregularVerb {
        v1: "sell",
        v2: "sold",
        v3: "sold",
        meaning_vi: "bán hàng",
    },
    IrregularVerb {
        v1: "send",
        v2: "sent",
        v3: "sent",
        meaning_vi: "gửi đi",
    },
    IrregularVerb {
        v1: "set",
        v2: "set",
        v3: "set",
        meaning_vi: "thiết lập, cài",
    },
    IrregularVerb {
        v1: "sew",
        v2: "sewed",
        v3: "sewn, sewed",
        meaning_vi: "may vá",
    },
    IrregularVerb {
        v1: "shake",
        v2: "shook",
        v3: "shaken",
        meaning_vi: "rung, lắc",
    },
    IrregularVerb {
        v1: "shall",
        v2: "should",
        v3: "-",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "shave",
        v2: "shaved",
        v3: "shaved, shaven",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "shear",
        v2: "sheared",
        v3: "shorn, sheared",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "shed",
        v2: "shed",
        v3: "shed",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "shine",
        v2: "shone, shined",
        v3: "shone, shined",
        meaning_vi: "chiếu sáng",
    },
    IrregularVerb {
        v1: "shoe",
        v2: "shod, shoed",
        v3: "shod, shoed",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "shoot",
        v2: "shot",
        v3: "shot",
        meaning_vi: "bắn súng",
    },
    IrregularVerb {
        v1: "show",
        v2: "showed",
        v3: "shown, showed",
        meaning_vi: "cho thấy, trình diễn",
    },
    IrregularVerb {
        v1: "shred",
        v2: "shredded, shred",
        v3: "shredded, shred",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "shrink",
        v2: "shrank, shrunk",
        v3: "shrunk, shrunken",
        meaning_vi: "co rút lại",
    },
    IrregularVerb {
        v1: "shrive",
        v2: "shrove, shrived",
        v3: "shriven, shrived",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "shut",
        v2: "shut",
        v3: "shut",
        meaning_vi: "đóng chặt",
    },
    IrregularVerb {
        v1: "sing",
        v2: "sang, sung",
        v3: "sung",
        meaning_vi: "ca hát",
    },
    IrregularVerb {
        v1: "sink",
        v2: "sank, sunk",
        v3: "sunk, sunken",
        meaning_vi: "chìm đắm",
    },
    IrregularVerb {
        v1: "sit",
        v2: "sat",
        v3: "sat",
        meaning_vi: "ngồi xuống",
    },
    IrregularVerb {
        v1: "slay",
        v2: "slew",
        v3: "slain",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "sleep",
        v2: "slept",
        v3: "slept",
        meaning_vi: "ngủ nghỉ",
    },
    IrregularVerb {
        v1: "slide",
        v2: "slid",
        v3: "slid, slidden",
        meaning_vi: "trượt, lướt",
    },
    IrregularVerb {
        v1: "sling",
        v2: "slung",
        v3: "slung",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "slink",
        v2: "slunk",
        v3: "slunk",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "slit",
        v2: "slit",
        v3: "slit",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "smell",
        v2: "smelt, smelled",
        v3: "smelt, smelled",
        meaning_vi: "ngửi thấy",
    },
    IrregularVerb {
        v1: "smite",
        v2: "smote",
        v3: "smitten, smote",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "sow",
        v2: "sowed",
        v3: "sown, sowed",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "speak",
        v2: "spoke",
        v3: "spoken",
        meaning_vi: "nói chuyện",
    },
    IrregularVerb {
        v1: "speed",
        v2: "sped, speeded",
        v3: "sped, speeded",
        meaning_vi: "tăng tốc",
    },
    IrregularVerb {
        v1: "spell",
        v2: "spelt, spelled",
        v3: "spelt, spelled",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "spend",
        v2: "spent",
        v3: "spent",
        meaning_vi: "tiêu sài, dành",
    },
    IrregularVerb {
        v1: "spill",
        v2: "spilt, spilled",
        v3: "spilt, spilled",
        meaning_vi: "làm tràn, đổ",
    },
    IrregularVerb {
        v1: "spin",
        v2: "spun",
        v3: "spun",
        meaning_vi: "quay tròn",
    },
    IrregularVerb {
        v1: "spit",
        v2: "spat, spit",
        v3: "spat, spit",
        meaning_vi: "nhổ nước bọt",
    },
    IrregularVerb {
        v1: "split",
        v2: "split",
        v3: "split",
        meaning_vi: "chia tách",
    },
    IrregularVerb {
        v1: "spoil",
        v2: "spoilt, spoiled",
        v3: "spoilt, spoiled",
        meaning_vi: "làm hỏng",
    },
    IrregularVerb {
        v1: "spread",
        v2: "spread",
        v3: "spread",
        meaning_vi: "lan tỏa, trải ra",
    },
    IrregularVerb {
        v1: "spring",
        v2: "sprang, sprung",
        v3: "sprung",
        meaning_vi: "bật nhảy",
    },
    IrregularVerb {
        v1: "stand",
        v2: "stood",
        v3: "stood",
        meaning_vi: "đứng vững",
    },
    IrregularVerb {
        v1: "stave",
        v2: "staved, stove",
        v3: "staved, stove",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "steal",
        v2: "stole",
        v3: "stolen",
        meaning_vi: "trộm cắp",
    },
    IrregularVerb {
        v1: "stick",
        v2: "stuck",
        v3: "stuck",
        meaning_vi: "dính vào, găm",
    },
    IrregularVerb {
        v1: "sting",
        v2: "stung",
        v3: "stung",
        meaning_vi: "chích, đốt",
    },
    IrregularVerb {
        v1: "stink",
        v2: "stank, stunk",
        v3: "stunk",
        meaning_vi: "bốc mùi hôi",
    },
    IrregularVerb {
        v1: "strew",
        v2: "strewed",
        v3: "strewn, strewed",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "stride",
        v2: "strode",
        v3: "stridden, strid",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "strike",
        v2: "struck",
        v3: "struck, stricken",
        meaning_vi: "đánh, tấn công",
    },
    IrregularVerb {
        v1: "string",
        v2: "strung",
        v3: "strung",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "strive",
        v2: "strove, strived",
        v3: "striven, strived",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "swear",
        v2: "swore",
        v3: "sworn",
        meaning_vi: "thề thốt",
    },
    IrregularVerb {
        v1: "sweat",
        v2: "sweat, sweated",
        v3: "sweat, sweated",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "sweep",
        v2: "swept",
        v3: "swept",
        meaning_vi: "quét dọn",
    },
    IrregularVerb {
        v1: "swell",
        v2: "swelled",
        v3: "swollen, swelled",
        meaning_vi: "sưng phù",
    },
    IrregularVerb {
        v1: "swim",
        v2: "swam",
        v3: "swum",
        meaning_vi: "bơi lội",
    },
    IrregularVerb {
        v1: "swing",
        v2: "swung",
        v3: "swung",
        meaning_vi: "đu đưa",
    },
    IrregularVerb {
        v1: "take",
        v2: "took",
        v3: "taken",
        meaning_vi: "cầm, lấy, mang",
    },
    IrregularVerb {
        v1: "teach",
        v2: "taught",
        v3: "taught",
        meaning_vi: "dạy học",
    },
    IrregularVerb {
        v1: "tear",
        v2: "tore",
        v3: "torn",
        meaning_vi: "xé rách",
    },
    IrregularVerb {
        v1: "tell",
        v2: "told",
        v3: "told",
        meaning_vi: "kể bảo",
    },
    IrregularVerb {
        v1: "think",
        v2: "thought",
        v3: "thought",
        meaning_vi: "suy nghĩ",
    },
    IrregularVerb {
        v1: "thrive",
        v2: "throve, thrived",
        v3: "thriven, thrived",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "throw",
        v2: "threw",
        v3: "thrown",
        meaning_vi: "ném, quăng",
    },
    IrregularVerb {
        v1: "thrust",
        v2: "thrust",
        v3: "thrust",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "tread",
        v2: "trod",
        v3: "trodden, trod",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "unbend",
        v2: "unbent",
        v3: "unbent",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "unbind",
        v2: "unbound",
        v3: "unbound",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "unbuild",
        v2: "unbuilt",
        v3: "unbuilt",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "underbid",
        v2: "underbid",
        v3: "underbidden, underbid",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "underbuy",
        v2: "underbought",
        v3: "underbought",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "undercut",
        v2: "undercut",
        v3: "undercut",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "underdo",
        v2: "underdid",
        v3: "underdone",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "underfeed",
        v2: "underfed",
        v3: "underfed",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "undergo",
        v2: "underwent",
        v3: "undergone",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "underlay",
        v2: "underlaid",
        v3: "underlaid",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "underlet",
        v2: "underlet",
        v3: "underlet",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "underlie",
        v2: "underlay",
        v3: "underlain",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "underpay",
        v2: "underpaid",
        v3: "underpaid",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "underrun",
        v2: "underran",
        v3: "underrun",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "undersell",
        v2: "undersold",
        v3: "undersold",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "underset",
        v2: "underset",
        v3: "underset",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "undershoot",
        v2: "undershot",
        v3: "undershot",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "understand",
        v2: "understood",
        v3: "understood",
        meaning_vi: "thấu hiểu",
    },
    IrregularVerb {
        v1: "undertake",
        v2: "undertook",
        v3: "undertaken",
        meaning_vi: "đảm nhận",
    },
    IrregularVerb {
        v1: "underwrite",
        v2: "underwrote",
        v3: "underwritten",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "undo",
        v2: "undid",
        v3: "undone",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "undraw",
        v2: "undrew",
        v3: "undrawn",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "unfreeze",
        v2: "unfroze",
        v3: "unfrozen",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "ungird",
        v2: "ungirded, ungirt",
        v3: "ungirded, ungirt",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "unhang",
        v2: "unhung",
        v3: "unhung",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "unknit",
        v2: "unknitted, unknit",
        v3: "unknitted, unknit",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "unlade",
        v2: "unladed",
        v3: "unladed, unladen",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "unlay",
        v2: "unlaid",
        v3: "unlaid",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "unlearn",
        v2: "unlearnt, unlearned",
        v3: "unlearnt, unlearned",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "unmake",
        v2: "unmade",
        v3: "unmade",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "unreeve",
        v2: "unrove, unreeved",
        v3: "unrove, unreeved",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "unsay",
        v2: "unsaid",
        v3: "unsaid",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "unset",
        v2: "unset",
        v3: "unset",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "unsling",
        v2: "unslung",
        v3: "unslung",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "unspeak",
        v2: "unspoke",
        v3: "unspoken",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "unstick",
        v2: "unstuck",
        v3: "unstuck",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "unstring",
        v2: "unstrung",
        v3: "unstrung",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "unswear",
        v2: "unswore",
        v3: "unsworn",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "unteach",
        v2: "untaught",
        v3: "untaught",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "unthink",
        v2: "unthought",
        v3: "unthought",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "untread",
        v2: "untrod",
        v3: "untrodden, untrod",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "unweave",
        v2: "unwove",
        v3: "unwoven",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "unwind",
        v2: "unwound",
        v3: "unwound",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "upbuild",
        v2: "upbuilt",
        v3: "upbuilt",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "upcast",
        v2: "upcast",
        v3: "upcast",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "uphold",
        v2: "upheld",
        v3: "upheld",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "uppercut",
        v2: "uppercut",
        v3: "uppercut",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "uprise",
        v2: "uprose",
        v3: "uprisen",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "upset",
        v2: "upset",
        v3: "upset",
        meaning_vi: "làm buồn bã",
    },
    IrregularVerb {
        v1: "upsweep",
        v2: "upswept",
        v3: "upswept",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "upswing",
        v2: "upswung",
        v3: "upswung",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "wake",
        v2: "waked, woke",
        v3: "waked, woken, woke",
        meaning_vi: "thức giấc",
    },
    IrregularVerb {
        v1: "waylay",
        v2: "waylaid",
        v3: "waylaid",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "wear",
        v2: "wore",
        v3: "worn",
        meaning_vi: "mặc, đeo",
    },
    IrregularVerb {
        v1: "weave",
        v2: "wove",
        v3: "woven",
        meaning_vi: "dệt vải",
    },
    IrregularVerb {
        v1: "wed",
        v2: "wedded, wed",
        v3: "wedded, wed",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "weep",
        v2: "wept",
        v3: "wept",
        meaning_vi: "khóc lóc",
    },
    IrregularVerb {
        v1: "wet",
        v2: "wetted, wet",
        v3: "wetted, wet",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "will",
        v2: "would",
        v3: "-",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "win",
        v2: "won",
        v3: "won",
        meaning_vi: "chiến thắng",
    },
    IrregularVerb {
        v1: "wind",
        v2: "winded, wound",
        v3: "winded, wound",
        meaning_vi: "uốn lượn, lên dây",
    },
    IrregularVerb {
        v1: "wit",
        v2: "wist",
        v3: "wist",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "withdraw",
        v2: "withdrew",
        v3: "withdrawn",
        meaning_vi: "rút lui, rút tiền",
    },
    IrregularVerb {
        v1: "withhold",
        v2: "withheld",
        v3: "withheld",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "withstand",
        v2: "withstood",
        v3: "withstood",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "work",
        v2: "worked",
        v3: "worked",
        meaning_vi: "",
    },
    IrregularVerb {
        v1: "wring",
        v2: "wrung",
        v3: "wrung",
        meaning_vi: "vắt kiệt",
    },
    IrregularVerb {
        v1: "write",
        v2: "wrote",
        v3: "written",
        meaning_vi: "viết chữ",
    },
];

pub fn list_irregular_verbs() -> &'static [IrregularVerb] {
    IRREGULAR_VERBS
}

pub fn search_irregular_verbs(query: &str) -> Vec<&'static IrregularVerb> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return IRREGULAR_VERBS.iter().collect();
    }
    IRREGULAR_VERBS
        .iter()
        .filter(|verb| {
            verb.v1.to_lowercase().contains(&q)
                || verb.v2.to_lowercase().contains(&q)
                || verb.v3.to_lowercase().contains(&q)
                || verb.meaning_vi.to_lowercase().contains(&q)
        })
        .collect()
}

// --- Unit Converter ---
pub fn convert_units(category: &str, value: f64, from: &str, to: &str) -> Option<f64> {
    match category {
        "length" => {
            // Base: meters
            let to_meters = match from {
                "m" => 1.0,
                "km" => 1000.0,
                "cm" => 0.01,
                "mm" => 0.001,
                "inch" => 0.0254,
                "foot" => 0.3048,
                "yard" => 0.9144,
                "mile" => 1609.344,
                _ => return None,
            };
            let from_meters = match to {
                "m" => 1.0,
                "km" => 0.001,
                "cm" => 100.0,
                "mm" => 1000.0,
                "inch" => 39.37007874,
                "foot" => 3.280839895,
                "yard" => 1.093613298,
                "mile" => 0.000621371,
                _ => return None,
            };
            Some(value * to_meters * from_meters)
        }
        "weight" => {
            // Base: kilograms
            let to_kg = match from {
                "kg" => 1.0,
                "g" => 0.001,
                "mg" => 0.000001,
                "lb" => 0.45359237,
                "oz" => 0.028349523,
                "ton" => 1000.0,
                _ => return None,
            };
            let from_kg = match to {
                "kg" => 1.0,
                "g" => 1000.0,
                "mg" => 1000000.0,
                "lb" => 2.20462262,
                "oz" => 35.2739619,
                "ton" => 0.001,
                _ => return None,
            };
            Some(value * to_kg * from_kg)
        }
        "temperature" => {
            // Convert to Celsius first
            let celsius = match from {
                "C" => value,
                "F" => (value - 32.0) * 5.0 / 9.0,
                "K" => value - 273.15,
                _ => return None,
            };
            match to {
                "C" => Some(celsius),
                "F" => Some(celsius * 9.0 / 5.0 + 32.0),
                "K" => Some(celsius + 273.15),
                _ => None,
            }
        }
        "volume" => {
            // Base: liters
            let to_liters = match from {
                "L" => 1.0,
                "mL" => 0.001,
                "gal" => 3.785411784,
                "fl_oz" => 0.029573529,
                _ => return None,
            };
            let from_liters = match to {
                "L" => 1.0,
                "mL" => 1000.0,
                "gal" => 0.264172052,
                "fl_oz" => 33.8140227,
                _ => return None,
            };
            Some(value * to_liters * from_liters)
        }
        _ => None,
    }
}

// --- IPA Phonetics ---
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpaSymbol {
    pub symbol: &'static str,
    pub category: &'static str,
    pub sample_word: &'static str,
    pub sample_ipa: &'static str,
    pub description_vi: &'static str,
}

pub static IPA_SYMBOLS: &[IpaSymbol] = &[
    // Short Vowels
    IpaSymbol {
        symbol: "ɪ",
        category: "Nguyên âm ngắn",
        sample_word: "sit",
        sample_ipa: "/sɪt/",
        description_vi: "Ngồi",
    },
    IpaSymbol {
        symbol: "e",
        category: "Nguyên âm ngắn",
        sample_word: "bed",
        sample_ipa: "/bed/",
        description_vi: "Cái giường",
    },
    IpaSymbol {
        symbol: "æ",
        category: "Nguyên âm ngắn",
        sample_word: "cat",
        sample_ipa: "/kæt/",
        description_vi: "Con mèo",
    },
    IpaSymbol {
        symbol: "ɒ",
        category: "Nguyên âm ngắn",
        sample_word: "hot",
        sample_ipa: "/hɒt/",
        description_vi: "Nóng",
    },
    IpaSymbol {
        symbol: "ʌ",
        category: "Nguyên âm ngắn",
        sample_word: "cup",
        sample_ipa: "/kʌp/",
        description_vi: "Cái cốc",
    },
    IpaSymbol {
        symbol: "ʊ",
        category: "Nguyên âm ngắn",
        sample_word: "book",
        sample_ipa: "/bʊk/",
        description_vi: "Cuốn sách",
    },
    IpaSymbol {
        symbol: "ə",
        category: "Nguyên âm ngắn",
        sample_word: "banana",
        sample_ipa: "/bəˈnɑːnə/",
        description_vi: "Quả chuối",
    },
    // Long Vowels
    IpaSymbol {
        symbol: "iː",
        category: "Nguyên âm dài",
        sample_word: "see",
        sample_ipa: "/siː/",
        description_vi: "Nhìn thấy",
    },
    IpaSymbol {
        symbol: "ɑː",
        category: "Nguyên âm dài",
        sample_word: "car",
        sample_ipa: "/kɑːr/",
        description_vi: "Xe hơi",
    },
    IpaSymbol {
        symbol: "ɔː",
        category: "Nguyên âm dài",
        sample_word: "door",
        sample_ipa: "/dɔːr/",
        description_vi: "Cửa ra vào",
    },
    IpaSymbol {
        symbol: "uː",
        category: "Nguyên âm dài",
        sample_word: "too",
        sample_ipa: "/tuː/",
        description_vi: "Quá, cũng",
    },
    IpaSymbol {
        symbol: "ɜː",
        category: "Nguyên âm dài",
        sample_word: "bird",
        sample_ipa: "/bɜːd/",
        description_vi: "Con chim",
    },
    // Diphthongs
    IpaSymbol {
        symbol: "eɪ",
        category: "Nguyên âm đôi",
        sample_word: "day",
        sample_ipa: "/deɪ/",
        description_vi: "Ngày",
    },
    IpaSymbol {
        symbol: "aɪ",
        category: "Nguyên âm đôi",
        sample_word: "sky",
        sample_ipa: "/skaɪ/",
        description_vi: "Bầu trời",
    },
    IpaSymbol {
        symbol: "ɔɪ",
        category: "Nguyên âm đôi",
        sample_word: "boy",
        sample_ipa: "/bɔɪ/",
        description_vi: "Cậu bé",
    },
    IpaSymbol {
        symbol: "aʊ",
        category: "Nguyên âm đôi",
        sample_word: "cow",
        sample_ipa: "/kaʊ/",
        description_vi: "Con bò",
    },
    IpaSymbol {
        symbol: "əʊ",
        category: "Nguyên âm đôi",
        sample_word: "go",
        sample_ipa: "/ɡəʊ/",
        description_vi: "Đi",
    },
    IpaSymbol {
        symbol: "ɪə",
        category: "Nguyên âm đôi",
        sample_word: "near",
        sample_ipa: "/nɪər/",
        description_vi: "Gần gũi",
    },
    IpaSymbol {
        symbol: "eə",
        category: "Nguyên âm đôi",
        sample_word: "hair",
        sample_ipa: "/heər/",
        description_vi: "Mái tóc",
    },
    IpaSymbol {
        symbol: "ʊə",
        category: "Nguyên âm đôi",
        sample_word: "tour",
        sample_ipa: "/tʊər/",
        description_vi: "Du lịch",
    },
    // Consonants (Voiceless)
    IpaSymbol {
        symbol: "p",
        category: "Phụ âm vô thanh",
        sample_word: "pen",
        sample_ipa: "/pen/",
        description_vi: "Cây bút",
    },
    IpaSymbol {
        symbol: "t",
        category: "Phụ âm vô thanh",
        sample_word: "tea",
        sample_ipa: "/tiː/",
        description_vi: "Trà",
    },
    IpaSymbol {
        symbol: "k",
        category: "Phụ âm vô thanh",
        sample_word: "cat",
        sample_ipa: "/kæt/",
        description_vi: "Mèo",
    },
    IpaSymbol {
        symbol: "f",
        category: "Phụ âm vô thanh",
        sample_word: "four",
        sample_ipa: "/fɔːr/",
        description_vi: "Số bốn",
    },
    IpaSymbol {
        symbol: "θ",
        category: "Phụ âm vô thanh",
        sample_word: "thin",
        sample_ipa: "/θɪn/",
        description_vi: "Mỏng manh",
    },
    IpaSymbol {
        symbol: "s",
        category: "Phụ âm vô thanh",
        sample_word: "see",
        sample_ipa: "/siː/",
        description_vi: "Nhìn",
    },
    IpaSymbol {
        symbol: "ʃ",
        category: "Phụ âm vô thanh",
        sample_word: "she",
        sample_ipa: "/ʃiː/",
        description_vi: "Cô ấy",
    },
    IpaSymbol {
        symbol: "tʃ",
        category: "Phụ âm vô thanh",
        sample_word: "chin",
        sample_ipa: "/tʃɪn/",
        description_vi: "Cằm",
    },
    // Consonants (Voiced)
    IpaSymbol {
        symbol: "b",
        category: "Phụ âm hữu thanh",
        sample_word: "bed",
        sample_ipa: "/bed/",
        description_vi: "Giường",
    },
    IpaSymbol {
        symbol: "d",
        category: "Phụ âm hữu thanh",
        sample_word: "dog",
        sample_ipa: "/dɒɡ/",
        description_vi: "Chó",
    },
    IpaSymbol {
        symbol: "ɡ",
        category: "Phụ âm hữu thanh",
        sample_word: "go",
        sample_ipa: "/ɡəʊ/",
        description_vi: "Đi",
    },
    IpaSymbol {
        symbol: "v",
        category: "Phụ âm hữu thanh",
        sample_word: "van",
        sample_ipa: "/væn/",
        description_vi: "Xe tải nhỏ",
    },
    IpaSymbol {
        symbol: "ð",
        category: "Phụ âm hữu thanh",
        sample_word: "this",
        sample_ipa: "/ðɪs/",
        description_vi: "Cái này",
    },
    IpaSymbol {
        symbol: "z",
        category: "Phụ âm hữu thanh",
        sample_word: "zoo",
        sample_ipa: "/zuː/",
        description_vi: "Sở thú",
    },
    IpaSymbol {
        symbol: "ʒ",
        category: "Phụ âm hữu thanh",
        sample_word: "vision",
        sample_ipa: "/ˈvɪʒn/",
        description_vi: "Tầm nhìn",
    },
    IpaSymbol {
        symbol: "dʒ",
        category: "Phụ âm hữu thanh",
        sample_word: "jump",
        sample_ipa: "/dʒʌmp/",
        description_vi: "Nhảy",
    },
    // Other Consonants
    IpaSymbol {
        symbol: "m",
        category: "Phụ âm mũi/khác",
        sample_word: "man",
        sample_ipa: "/mæn/",
        description_vi: "Người đàn ông",
    },
    IpaSymbol {
        symbol: "n",
        category: "Phụ âm mũi/khác",
        sample_word: "no",
        sample_ipa: "/nəʊ/",
        description_vi: "Không",
    },
    IpaSymbol {
        symbol: "ŋ",
        category: "Phụ âm mũi/khác",
        sample_word: "sing",
        sample_ipa: "/sɪŋ/",
        description_vi: "Hát",
    },
    IpaSymbol {
        symbol: "h",
        category: "Phụ âm mũi/khác",
        sample_word: "hat",
        sample_ipa: "/hæt/",
        description_vi: "Cái nón",
    },
    IpaSymbol {
        symbol: "l",
        category: "Phụ âm mũi/khác",
        sample_word: "leg",
        sample_ipa: "/leɡ/",
        description_vi: "Cái chân",
    },
    IpaSymbol {
        symbol: "r",
        category: "Phụ âm mũi/khác",
        sample_word: "red",
        sample_ipa: "/red/",
        description_vi: "Màu đỏ",
    },
    IpaSymbol {
        symbol: "w",
        category: "Phụ âm mũi/khác",
        sample_word: "wet",
        sample_ipa: "/wet/",
        description_vi: "Ẩm ướt",
    },
    IpaSymbol {
        symbol: "j",
        category: "Phụ âm mũi/khác",
        sample_word: "yes",
        sample_ipa: "/jes/",
        description_vi: "Đúng vậy",
    },
];

// --- Dialing Codes ---
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialingCode {
    pub country_vi: &'static str,
    pub country_en: &'static str,
    pub code: &'static str,
    pub iso: &'static str,
}

pub static DIALING_CODES: &[DialingCode] = &[
    DialingCode {
        country_vi: "Việt Nam",
        country_en: "Vietnam",
        code: "+84",
        iso: "VN",
    },
    DialingCode {
        country_vi: "Hoa Kỳ",
        country_en: "United States",
        code: "+1",
        iso: "US",
    },
    DialingCode {
        country_vi: "Canada",
        country_en: "Canada",
        code: "+1",
        iso: "CA",
    },
    DialingCode {
        country_vi: "Vương quốc Anh",
        country_en: "United Kingdom",
        code: "+44",
        iso: "GB",
    },
    DialingCode {
        country_vi: "Nhật Bản",
        country_en: "Japan",
        code: "+81",
        iso: "JP",
    },
    DialingCode {
        country_vi: "Hàn Quốc",
        country_en: "South Korea",
        code: "+82",
        iso: "KR",
    },
    DialingCode {
        country_vi: "Trung Quốc",
        country_en: "China",
        code: "+86",
        iso: "CN",
    },
    DialingCode {
        country_vi: "Pháp",
        country_en: "France",
        code: "+33",
        iso: "FR",
    },
    DialingCode {
        country_vi: "Đức",
        country_en: "Germany",
        code: "+49",
        iso: "DE",
    },
    DialingCode {
        country_vi: "Úc",
        country_en: "Australia",
        code: "+61",
        iso: "AU",
    },
    DialingCode {
        country_vi: "Singapore",
        country_en: "Singapore",
        code: "+65",
        iso: "SG",
    },
    DialingCode {
        country_vi: "Thái Lan",
        country_en: "Thailand",
        code: "+66",
        iso: "TH",
    },
    DialingCode {
        country_vi: "Malaysia",
        country_en: "Malaysia",
        code: "+60",
        iso: "MY",
    },
    DialingCode {
        country_vi: "Indonesia",
        country_en: "Indonesia",
        code: "+62",
        iso: "ID",
    },
    DialingCode {
        country_vi: "Philippines",
        country_en: "Philippines",
        code: "+63",
        iso: "PH",
    },
    DialingCode {
        country_vi: "Ấn Độ",
        country_en: "India",
        code: "+91",
        iso: "IN",
    },
    DialingCode {
        country_vi: "Nga",
        country_en: "Russia",
        code: "+7",
        iso: "RU",
    },
    DialingCode {
        country_vi: "Ý",
        country_en: "Italy",
        code: "+39",
        iso: "IT",
    },
    DialingCode {
        country_vi: "Tây Ban Nha",
        country_en: "Spain",
        code: "+34",
        iso: "ES",
    },
    DialingCode {
        country_vi: "Hà Lan",
        country_en: "Netherlands",
        code: "+31",
        iso: "NL",
    },
    DialingCode {
        country_vi: "Thụy Sĩ",
        country_en: "Switzerland",
        code: "+41",
        iso: "CH",
    },
    DialingCode {
        country_vi: "Thụy Điển",
        country_en: "Sweden",
        code: "+46",
        iso: "SE",
    },
    DialingCode {
        country_vi: "Na Uy",
        country_en: "Norway",
        code: "+47",
        iso: "NO",
    },
    DialingCode {
        country_vi: "Đan Mạch",
        country_en: "Denmark",
        code: "+45",
        iso: "DK",
    },
    DialingCode {
        country_vi: "Phần Lan",
        country_en: "Finland",
        code: "+358",
        iso: "FI",
    },
    DialingCode {
        country_vi: "Ba Lan",
        country_en: "Poland",
        code: "+48",
        iso: "PL",
    },
    DialingCode {
        country_vi: "Bỉ",
        country_en: "Belgium",
        code: "+32",
        iso: "BE",
    },
    DialingCode {
        country_vi: "Áo",
        country_en: "Austria",
        code: "+43",
        iso: "AT",
    },
    DialingCode {
        country_vi: "New Zealand",
        country_en: "New Zealand",
        code: "+64",
        iso: "NZ",
    },
    DialingCode {
        country_vi: "Đài Loan",
        country_en: "Taiwan",
        code: "+886",
        iso: "TW",
    },
    DialingCode {
        country_vi: "Hồng Kông",
        country_en: "Hong Kong",
        code: "+852",
        iso: "HK",
    },
    DialingCode {
        country_vi: "Lào",
        country_en: "Laos",
        code: "+856",
        iso: "LA",
    },
    DialingCode {
        country_vi: "Campuchia",
        country_en: "Cambodia",
        code: "+855",
        iso: "KH",
    },
    DialingCode {
        country_vi: "Myanmar",
        country_en: "Myanmar",
        code: "+95",
        iso: "MM",
    },
    DialingCode {
        country_vi: "Brazil",
        country_en: "Brazil",
        code: "+55",
        iso: "BR",
    },
    DialingCode {
        country_vi: "Mexico",
        country_en: "Mexico",
        code: "+52",
        iso: "MX",
    },
    DialingCode {
        country_vi: "Argentina",
        country_en: "Argentina",
        code: "+54",
        iso: "AR",
    },
    DialingCode {
        country_vi: "Thổ Nhĩ Kỳ",
        country_en: "Turkey",
        code: "+90",
        iso: "TR",
    },
    DialingCode {
        country_vi: "Ai Cập",
        country_en: "Egypt",
        code: "+20",
        iso: "EG",
    },
    DialingCode {
        country_vi: "Nam Phi",
        country_en: "South Africa",
        code: "+27",
        iso: "ZA",
    },
    DialingCode {
        country_vi: "Ả Rập Xê Út",
        country_en: "Saudi Arabia",
        code: "+966",
        iso: "SA",
    },
    DialingCode {
        country_vi: "UAE",
        country_en: "United Arab Emirates",
        code: "+971",
        iso: "AE",
    },
    DialingCode {
        country_vi: "Israel",
        country_en: "Israel",
        code: "+972",
        iso: "IL",
    },
];

pub fn search_dialing_codes(query: &str) -> Vec<&'static DialingCode> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return DIALING_CODES.iter().collect();
    }
    DIALING_CODES
        .iter()
        .filter(|dc| {
            dc.country_vi.to_lowercase().contains(&q)
                || dc.country_en.to_lowercase().contains(&q)
                || dc.code.contains(&q)
                || dc.iso.to_lowercase().contains(&q)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_irregular_verbs_search() {
        let verbs = list_irregular_verbs();
        assert!(verbs.len() >= 350);

        let go_results = search_irregular_verbs("go");
        assert!(!go_results.is_empty());
        assert!(
            go_results
                .iter()
                .any(|v| v.v1 == "go" && v.v2 == "went" && v.v3 == "gone")
        );

        let vietnamese_meaning_search = search_irregular_verbs("viết");
        assert!(vietnamese_meaning_search.iter().any(|v| v.v1 == "write"));
    }

    #[test]
    fn test_unit_conversions() {
        // Length
        let km = convert_units("length", 1000.0, "m", "km").unwrap();
        assert!((km - 1.0).abs() < 1e-6);

        // Weight
        let lb = convert_units("weight", 1.0, "kg", "lb").unwrap();
        assert!((lb - 2.20462).abs() < 1e-3);

        // Temperature
        let fahrenheit = convert_units("temperature", 100.0, "C", "F").unwrap();
        assert!((fahrenheit - 212.0).abs() < 1e-6);
        let kelvin = convert_units("temperature", 0.0, "C", "K").unwrap();
        assert!((kelvin - 273.15).abs() < 1e-6);

        // Volume
        let ml = convert_units("volume", 2.5, "L", "mL").unwrap();
        assert!((ml - 2500.0).abs() < 1e-6);

        // Invalid unit/category returns None
        assert!(convert_units("invalid", 10.0, "m", "km").is_none());
        assert!(convert_units("length", 10.0, "m", "unknown").is_none());
    }

    #[test]
    fn test_dialing_codes_search() {
        let vn = search_dialing_codes("84");
        assert!(vn.iter().any(|d| d.iso == "VN" && d.code == "+84"));

        let us = search_dialing_codes("United States");
        assert!(us.iter().any(|d| d.iso == "US" && d.code == "+1"));
    }

    #[test]
    fn test_ipa_symbols_complete() {
        assert_eq!(IPA_SYMBOLS.len(), 44);
        assert!(IPA_SYMBOLS.iter().any(|s| s.symbol == "iː"));
        assert!(IPA_SYMBOLS.iter().any(|s| s.symbol == "ʃ"));
    }
}

// Explicit people in curated credits and reviewed source bylines. Unresolved credits are excluded.
// Reviewed minimum, not an exhaustive census. Van K. Tharp and Van Tharp are the same person.
export const authorIdentities: Record<string, readonly string[]> = {
  "connors": [
    "Blake Hayward",
    "Cesar Alvarez",
    "Chris Cain",
    "Larry Connors",
    "Linda Raschke",
    "Mark Boucher"
  ],
  "chande": [
    "Tushar Chande"
  ],
  "kroll": [
    "Stanley Kroll"
  ],
  "abell": [
    "Howard Abell"
  ],
  "radge": [
    "Nick Radge"
  ],
  "covel": [
    "Michael Covel"
  ],
  "tharp": [
    "Brian June",
    "D. R. Barton Jr.",
    "Henry O. Pruden",
    "John Burley",
    "Steve Sjuggerud",
    "Van Tharp"
  ],
  "lwilliams": [
    "Larry Williams"
  ],
  "crabel": [
    "Toby Crabel"
  ],
  "kaufman": [
    "Perry Kaufman"
  ],
  "harris": [
    "Michael Harris"
  ],
  "cooper": [
    "Jeff Cooper"
  ],
  "stridsman": [
    "Thomas Stridsman"
  ],
  "jaekle": [
    "Emilio Tomasini",
    "Urban Jaekle"
  ],
  "appel": [
    "Gerald Appel"
  ],
  "pruitt": [
    "George Pruitt",
    "John Hill"
  ],
  "lebeau": [
    "Charles LeBeau",
    "David Lucas"
  ],
  "stad": [],
  "katz": [],
  "bernstein": [
    "Jake Bernstein"
  ],
  "ehlers": [
    "John Ehlers"
  ],
  "demark": [
    "Tom DeMark"
  ],
  "babcock": [
    "Bruce Babcock"
  ],
  "pardo": [
    "Robert Pardo"
  ],
  "hirsch": [
    "Jeffrey Hirsch",
    "Yale Hirsch"
  ],
  "krutsinger": [
    "Joe Krutsinger"
  ],
  "dunnigan": [
    "William Dunnigan"
  ],
  "ruggiero": [
    "Murray Ruggiero"
  ],
  "momentum": [],
  "bwilliams": [
    "Bill Williams"
  ],
  "landry": [
    "Dave Landry"
  ],
  "boucher": [
    "Mark Boucher"
  ],
  "katsanos": [
    "Markos Katsanos"
  ],
  "weissman": [
    "Richard Weissman"
  ],
  "bandy": [
    "Howard Bandy"
  ],
  "meyers": [
    "Dennis Meyers"
  ],
  "clayburg": [
    "John Clayburg"
  ],
  "wilder": [
    "J. Welles Wilder"
  ],
  "bollinger": [
    "John Bollinger"
  ],
  "elder": [
    "Alexander Elder"
  ],
  "primo": [
    "Steven Primo"
  ],
  "weinstein": [
    "Stan Weinstein"
  ],
  "guppy": [
    "Daryl Guppy"
  ],
  "haggerty": [
    "Cynthia Kase",
    "Kevin Haggerty"
  ],
  "quant": [],
  "collins": [
    "Art Collins"
  ],
  "kestner": [
    "Lars Kestner"
  ],
  "seasonal": [
    "Jay Kaeppel",
    "Marvin Appel"
  ],
  "freeburg": [
    "Nelson Freeburg"
  ],
  "gettess": [
    "Lee Gettess",
    "Peter Aan"
  ],
  "lukac": [],
  "timers": [
    "Ned Davis",
    "Ryan Jones"
  ],
  "evidence": [],
  "blau": [
    "Tom Bierovic",
    "William Blau"
  ],
  "sperandeo": [
    "David Vomund",
    "Victor Sperandeo"
  ],
  "berg": [
    "Jim Berg",
    "Richard Anderson"
  ],
  "mtiming": [],
  "pring": [
    "Martin J. Pring"
  ],
  "gregoriou": [
    "Greg N. Gregoriou"
  ],
  "crisp": [
    "Mark Crisp"
  ],
  "fabozzi": [
    "Frank J. Fabozzi"
  ],
  "fontanills": [
    "George A. Fontanills"
  ],
  "thomsett": [
    "Michael C. Thomsett"
  ],
  "fischer": [
    "Robert Fischer"
  ],
  "patel": [
    "Alpesh B. Patel"
  ],
  "vanvliet": [
    "Benjamin Van Vliet"
  ],
  "carney": [
    "Scott M. Carney"
  ],
  "etzkorn": [
    "Mark Etzkorn"
  ],
  "bickford": [
    "James L. Bickford"
  ],
  "mdarcher": [
    "Michael Duane Archer"
  ],
  "gbayer": [
    "George Bayer"
  ],
  "person": [
    "John L. Person"
  ],
  "bigalow": [
    "Stephen W. Bigalow"
  ],
  "murphy": [
    "John J. Murphy"
  ],
  "pesavento": [
    "Larry Pesavento"
  ],
  "sysmisc": [],
  "swingstk": [],
  "vendors": [],
  "scart": [],
  "swing12": [],
  "schwager": [
    "Jack D. Schwager"
  ],
  "magazines": [],
  "t2wgrp": [],
  "v6misc": [],
  "vcode": [],
  "ross": [
    "Joe Ross"
  ],
  "raschke": [
    "Linda Raschke"
  ],
  "minervini": [
    "Mark Minervini"
  ],
  "oneil": [
    "William O'Neil"
  ],
  "darvas": [
    "Nicolas Darvas"
  ],
  "livermore": [
    "Edwin LeFevre",
    "Jesse Livermore"
  ],
  "pristine": [
    "Greg Capra",
    "Oliver Velez"
  ]
};
export const identifiedAuthorCount = new Set(Object.values(authorIdentities).flat()).size;

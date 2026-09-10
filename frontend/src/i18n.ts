// Translation catalogue for the tasks module, one file per language under
// `locales/`. `en` is the source of truth (see locales/en.ts); each other
// language is typed against it. Split out of a single 2000-line file so a
// translator touches only their own language and sessions don't collide.
import { registerModuleTranslations } from '@kubuno/sdk'
import { en } from './locales/en'
import { fr } from './locales/fr'
import { es } from './locales/es'
import { pt } from './locales/pt'
import { it } from './locales/it'
import { de } from './locales/de'
import { el } from './locales/el'
import { ru } from './locales/ru'
import { ar } from './locales/ar'
import { he } from './locales/he'
import { hi } from './locales/hi'
import { zh } from './locales/zh'
import { ja } from './locales/ja'

registerModuleTranslations('tasks', { en, fr, es, pt, it, de, el, ru, ar, he, hi, zh, ja })

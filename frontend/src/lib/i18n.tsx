import * as React from 'react'

/**
 * Minimal i18n: a typed English dictionary with per-locale overrides.
 * Only the sign-in and password pages are translated so far; add keys here as more screens are localized.
 */
export const LOCALES = [
  { code: 'en', label: 'English', short: 'EN' },
  { code: 'tr', label: 'Türkçe', short: 'TR' },
] as const
export type Locale = (typeof LOCALES)[number]['code']

const en = {
  'auth.brand.heading': 'Learn step by step,',
  'auth.brand.headingAccent': 'week by week.',
  'auth.brand.subtitle': 'Your course, assignments, feedback and progress in one place.',
  'auth.brand.h1.title': 'Week-by-week curriculum',
  'auth.brand.h1.text': 'Lessons, labs and resources laid out in the order you learn them.',
  'auth.brand.h2.title': 'Submissions and feedback',
  'auth.brand.h2.text': 'Hand in your work and get graded reviews from your instructor.',
  'auth.brand.h3.title': 'Clear progress',
  'auth.brand.h3.text': 'See what is done, what is next and how you are doing.',
  'auth.brand.footer': 'Open-source learning platform',
  'auth.brand.poweredBy': 'Powered by OpenSyllabus',
  'auth.help': 'Need help? Contact',

  'login.title': 'Sign in',
  'login.description': 'Welcome back. Sign in to continue learning.',
  'login.identifier': 'Email or username',
  'login.identifier.required': 'Enter your email or username',
  'login.password': 'Password',
  'login.password.required': 'Enter your password',
  'login.submit': 'Sign in',
  'login.forgot': 'Forgot your password? Ask your instructor for a reset link.',

  'reset.title.setup': 'Set your password',
  'reset.title.reset': 'Reset your password',
  'reset.incomplete': 'This link is incomplete.',
  'reset.askNewLink': 'Ask your instructor for a new link.',
  'reset.done.title': 'Password updated',
  'reset.done.description': 'You can now sign in with your new password.',
  'reset.done.continue': 'Continue to sign in',
  'reset.description': 'Choose a strong password of at least 10 characters.',
  'reset.new': 'New password',
  'reset.confirm': 'Confirm password',
  'reset.submit.setup': 'Set password',
  'reset.submit.reset': 'Update password',
  'reset.mismatch': 'Passwords do not match',
  'password.min': 'Use at least 10 characters',
  'password.max': 'Use at most 128 characters',
  'password.symbol': 'Include at least one number or symbol',

  'prefs.language': 'Language',
  'prefs.theme': 'Theme',
  'prefs.light': 'Light mode',
  'prefs.dark': 'Dark mode',
}

export type MessageKey = keyof typeof en

const tr: Partial<Record<MessageKey, string>> = {
  'auth.brand.heading': 'Adım adım,',
  'auth.brand.headingAccent': 'hafta hafta öğren.',
  'auth.brand.subtitle': 'Dersin, ödevlerin, geri bildirimlerin ve ilerlemen tek bir yerde.',
  'auth.brand.h1.title': 'Haftalık müfredat',
  'auth.brand.h1.text': 'Dersler, laboratuvarlar ve kaynaklar öğrenme sırasına göre düzenlenmiş.',
  'auth.brand.h2.title': 'Teslimler ve geri bildirim',
  'auth.brand.h2.text': 'Çalışmanı teslim et, eğitmeninden notlu değerlendirme al.',
  'auth.brand.h3.title': 'Net ilerleme',
  'auth.brand.h3.text': 'Neyi bitirdiğini, sırada ne olduğunu ve nasıl gittiğini gör.',
  'auth.brand.footer': 'Açık kaynak öğrenme platformu',
  'auth.brand.poweredBy': 'OpenSyllabus altyapısıyla',
  'auth.help': 'Yardım mı lazım? İletişim:',

  'login.title': 'Giriş yap',
  'login.description': 'Tekrar hoş geldin. Öğrenmeye devam etmek için giriş yap.',
  'login.identifier': 'E-posta veya kullanıcı adı',
  'login.identifier.required': 'E-posta adresini veya kullanıcı adını gir',
  'login.password': 'Şifre',
  'login.password.required': 'Şifreni gir',
  'login.submit': 'Giriş yap',
  'login.forgot': 'Şifreni mi unuttun? Eğitmeninden sıfırlama bağlantısı iste.',

  'reset.title.setup': 'Şifreni belirle',
  'reset.title.reset': 'Şifreni sıfırla',
  'reset.incomplete': 'Bu bağlantı eksik.',
  'reset.askNewLink': 'Eğitmeninden yeni bir bağlantı iste.',
  'reset.done.title': 'Şifre güncellendi',
  'reset.done.description': 'Artık yeni şifrenle giriş yapabilirsin.',
  'reset.done.continue': 'Girişe devam et',
  'reset.description': 'En az 10 karakterden oluşan güçlü bir şifre seç.',
  'reset.new': 'Yeni şifre',
  'reset.confirm': 'Şifreyi onayla',
  'reset.submit.setup': 'Şifreyi belirle',
  'reset.submit.reset': 'Şifreyi güncelle',
  'reset.mismatch': 'Şifreler eşleşmiyor',
  'password.min': 'En az 10 karakter kullan',
  'password.max': 'En fazla 128 karakter kullan',
  'password.symbol': 'En az bir rakam veya sembol ekle',

  'prefs.language': 'Dil',
  'prefs.theme': 'Tema',
  'prefs.light': 'Açık tema',
  'prefs.dark': 'Koyu tema',
}

const MESSAGES: Record<Locale, Partial<Record<MessageKey, string>>> = { en, tr }
const KEY = 'opensyllabus-locale'

function initialLocale(): Locale {
  try {
    const saved = localStorage.getItem(KEY)
    if (saved && LOCALES.some((l) => l.code === saved)) return saved as Locale
  } catch {
    /* storage unavailable */
  }
  return typeof navigator !== 'undefined' && navigator.language?.toLowerCase().startsWith('tr') ? 'tr' : 'en'
}

type Ctx = { locale: Locale; setLocale: (l: Locale) => void; t: (key: MessageKey) => string }
const LocaleContext = React.createContext<Ctx>({ locale: 'en', setLocale: () => {}, t: (k) => en[k] })

export function LocaleProvider({ children }: { children: React.ReactNode }) {
  const [locale, setLocaleState] = React.useState<Locale>(initialLocale)

  React.useEffect(() => {
    document.documentElement.lang = locale
  }, [locale])

  const setLocale = React.useCallback((l: Locale) => {
    try {
      localStorage.setItem(KEY, l)
    } catch {
      /* storage unavailable */
    }
    setLocaleState(l)
  }, [])

  const value = React.useMemo<Ctx>(() => ({ locale, setLocale, t: (key) => MESSAGES[locale][key] ?? en[key] }), [locale, setLocale])
  return <LocaleContext.Provider value={value}>{children}</LocaleContext.Provider>
}

export const useLocale = () => React.useContext(LocaleContext)
export const useT = () => React.useContext(LocaleContext).t

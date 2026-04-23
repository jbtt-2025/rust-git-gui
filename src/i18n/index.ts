import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import en from './en.json';
import zh_CN from './zh_CN.json';
import ja from './ja.json';

/** Detect the browser / OS language and map to a supported locale. */
function detectLanguage(): string {
  const lang = navigator.language ?? 'en';
  if (lang.startsWith('zh')) return 'zh_CN';
  if (lang.startsWith('ja')) return 'ja';
  return 'en';
}

i18n.use(initReactI18next).init({
  resources: {
    en: { translation: en },
    zh_CN: { translation: zh_CN },
    ja: { translation: ja },
  },
  lng: detectLanguage(),
  fallbackLng: 'en',
  interpolation: {
    escapeValue: false, // React already escapes
  },
});

/** Change the active language at runtime (no restart needed). */
export function changeLanguage(lang: string): Promise<void> {
  return i18n.changeLanguage(lang).then(() => undefined);
}

export default i18n;

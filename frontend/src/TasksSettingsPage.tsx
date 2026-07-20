import React, { useState } from 'react'
import { Link } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { CheckSquare, ArrowLeft, ExternalLink, Check } from 'lucide-react'
import { Toggle, Button, Radio } from '@ui'
import { useModulePrefs } from './userPrefs'
import TasksCalDavSettings from './TasksCalDavSettings'

// ── Per-user preferences (backend, cross-device via core users.preferences) ─────

interface TasksPrefs {
  defaultView:    string  // 'list' | 'kanban'
  defaultSort:    string  // 'manual' | 'due' | 'priority' | 'created' | 'title'
  hideCompleted:  boolean
  confirmDelete:  boolean
  groupBy:        string  // 'none' | 'priority' | 'due'
  [key: string]:  unknown // satisfies ModuleSettingsRegistry's Record constraint
}

const DEFAULT_PREFS: TasksPrefs = {
  defaultView: 'list', defaultSort: 'manual', hideCompleted: false,
  confirmDelete: true, groupBy: 'none',
}

// ── Mail-style layout helpers ───────────────────────────────────────────────────

function SettingsRow({ label, description, children }: {
  label: string; description?: string; children: React.ReactNode
}) {
  return (
    <div className="flex items-start gap-8 py-4 border-b border-[#e8eaed] last:border-0">
      <div className="w-60 flex-shrink-0">
        <p className="text-sm text-[#202124] font-normal">{label}</p>
        {description && <p className="text-xs text-text-tertiary mt-0.5 leading-relaxed">{description}</p>}
      </div>
      <div className="flex-1">{children}</div>
    </div>
  )
}

function RadioGroup({ options, value, onChange }: {
  options: { value: string; label: string }[]; value: string; onChange: (v: string) => void
}) {
  return (
    <div className="flex flex-col items-start gap-2">
      {options.map(opt => (
        <Radio key={opt.value} checked={value === opt.value} onChange={() => onChange(opt.value)} label={opt.label} />
      ))}
    </div>
  )
}

// ── Préférences tab (per-user) ──────────────────────────────────────────────────

function PreferencesTab() {
  const { t } = useTranslation('tasks')
  const { prefs: saved, update } = useModulePrefs<TasksPrefs>('tasks', DEFAULT_PREFS)
  const [prefs, setPrefs] = useState<TasksPrefs>(saved)
  const [savedFlag, setSavedFlag] = useState(false)
  const [busy, setBusy] = useState(false)

  const set = <K extends keyof TasksPrefs>(key: K, value: TasksPrefs[K]) =>
    setPrefs(p => ({ ...p, [key]: value }))

  const save = async () => {
    setBusy(true)
    try {
      await update(prefs)
      setSavedFlag(true)
      setTimeout(() => setSavedFlag(false), 2500)
    } finally { setBusy(false) }
  }

  return (
    <div>
      <SettingsRow
        label={t('tasks_pref_default_view', { defaultValue: 'Vue par défaut' })}
        description={t('tasks_pref_default_view_desc', { defaultValue: 'Affichage utilisé à l\'ouverture d\'un tableau.' })}
      >
        <RadioGroup
          value={prefs.defaultView}
          onChange={v => set('defaultView', v)}
          options={[
            { value: 'list',   label: t('tasks_pref_view_list',   { defaultValue: 'Liste' }) },
            { value: 'kanban', label: t('tasks_pref_view_kanban', { defaultValue: 'Tableau Kanban' }) },
          ]}
        />
      </SettingsRow>

      <SettingsRow
        label={t('tasks_pref_default_sort', { defaultValue: 'Tri par défaut' })}
        description={t('tasks_pref_default_sort_desc', { defaultValue: 'Ordre d\'affichage des tâches.' })}
      >
        <RadioGroup
          value={prefs.defaultSort}
          onChange={v => set('defaultSort', v)}
          options={[
            { value: 'manual',   label: t('tasks_pref_sort_manual',   { defaultValue: 'Manuel' }) },
            { value: 'due',      label: t('tasks_pref_sort_due',      { defaultValue: 'Échéance' }) },
            { value: 'priority', label: t('tasks_pref_sort_priority', { defaultValue: 'Priorité' }) },
            { value: 'created',  label: t('tasks_pref_sort_created',  { defaultValue: 'Date de création' }) },
            { value: 'title',    label: t('tasks_pref_sort_title',    { defaultValue: 'Titre' }) },
          ]}
        />
      </SettingsRow>

      <SettingsRow
        label={t('tasks_pref_group_by', { defaultValue: 'Regrouper par' })}
        description={t('tasks_pref_group_by_desc', { defaultValue: 'Regrouper les tâches sous des en-têtes.' })}
      >
        <RadioGroup
          value={prefs.groupBy}
          onChange={v => set('groupBy', v)}
          options={[
            { value: 'none',     label: t('tasks_pref_group_none',     { defaultValue: 'Aucun regroupement' }) },
            { value: 'priority', label: t('tasks_pref_group_priority', { defaultValue: 'Priorité' }) },
            { value: 'due',      label: t('tasks_pref_group_due',      { defaultValue: 'Échéance' }) },
          ]}
        />
      </SettingsRow>

      <SettingsRow label={t('tasks_pref_hide_completed', { defaultValue: 'Tâches terminées' })}>
        <label className="flex items-center gap-2 cursor-pointer select-none">
          <Toggle checked={prefs.hideCompleted} onChange={() => set('hideCompleted', !prefs.hideCompleted)} />
          <span className="text-sm text-text-primary">{t('tasks_pref_hide_completed_on', { defaultValue: 'Masquer les tâches terminées' })}</span>
        </label>
      </SettingsRow>

      <SettingsRow label={t('tasks_pref_confirm_delete', { defaultValue: 'Suppression' })}>
        <label className="flex items-center gap-2 cursor-pointer select-none">
          <Toggle checked={prefs.confirmDelete} onChange={() => set('confirmDelete', !prefs.confirmDelete)} />
          <span className="text-sm text-text-primary">{t('tasks_pref_confirm_delete_on', { defaultValue: 'Demander confirmation avant de supprimer' })}</span>
        </label>
      </SettingsRow>

      <div className="pt-5 flex items-center gap-3">
        <Button onClick={save} loading={busy}>
          {savedFlag
            ? <><Check size={14} className="mr-1.5 inline" />{t('tasks_settings_saved', { defaultValue: 'Enregistré' })}</>
            : t('tasks_settings_save_changes', { defaultValue: 'Enregistrer les modifications' })}
        </Button>
        <Button variant="ghost" onClick={() => setPrefs(saved)}>
          {t('common_cancel', { defaultValue: 'Annuler' })}
        </Button>
      </div>
    </div>
  )
}

// ── CalDAV tab (per-user, existing component) ───────────────────────────────────

function CalDavTab() {
  return (
    <div>
      <TasksCalDavSettings />
    </div>
  )
}

// ── À propos tab ────────────────────────────────────────────────────────────────

function AboutTab() {
  const { t } = useTranslation('tasks')
  return (
    <div className="rounded-xl border border-border overflow-hidden">
      <div className="flex items-center gap-3 px-5 py-4 border-b border-border bg-surface-1">
        <div className="w-10 h-10 rounded-xl bg-blue-100 flex items-center justify-center shrink-0">
          <CheckSquare size={20} className="text-blue-600" />
        </div>
        <div>
          <p className="text-sm font-semibold text-text-primary">Kubuno Tasks</p>
          <p className="text-xs text-text-tertiary">v0.1.0 · {t('tasks_official_module', { defaultValue: 'Module officiel' })}</p>
        </div>
        <span className="ml-auto text-xs font-medium px-2 py-0.5 rounded-full bg-orange-100 text-orange-700">Rust</span>
      </div>
      <div className="px-5 py-4">
        <a href="https://github.com/kubuno/kubuno" target="_blank" rel="noopener noreferrer"
          className="inline-flex items-center gap-1.5 text-sm text-primary hover:underline">
          <ExternalLink size={13} /> github.com/kubuno/kubuno
        </a>
      </div>
    </div>
  )
}

// ── Main page (mail-style breadcrumb + tab bar) ─────────────────────────────────

type Tab = 'preferences' | 'caldav' | 'about'

export default function TasksSettingsPage() {
  const { t } = useTranslation('tasks')
  const [tab, setTab] = useState<Tab>('preferences')

  const tabs: { id: Tab; label: string }[] = [
    { id: 'preferences', label: t('tasks_tab_preferences', { defaultValue: 'Préférences' }) },
    { id: 'caldav',      label: t('tasks_tab_caldav', { defaultValue: 'CalDAV' }) },
    { id: 'about',       label: t('tasks_tab_about', { defaultValue: 'À propos' }) },
  ]

  return (
    <div className="flex flex-col h-full bg-white overflow-hidden">
      {/* Breadcrumb header */}
      <div className="flex items-center gap-2 px-6 py-2.5 border-b border-[#e8eaed] flex-shrink-0" style={{ background: '#f8f9fa' }}>
        <Link to="/tasks" className="flex items-center gap-1.5 text-sm text-[#1a73e8] hover:underline">
          <ArrowLeft size={14} />
          Tasks
        </Link>
        <span className="text-text-tertiary text-sm">/</span>
        <div className="flex items-center gap-1.5">
          <CheckSquare size={15} className="text-text-secondary" />
          <span className="text-sm text-text-primary">{t('settings_title', { defaultValue: 'Réglages' })}</span>
        </div>
      </div>

      {/* Tab bar (Gmail-style) */}
      <div className="flex items-end border-b border-[#e8eaed] px-4 flex-shrink-0 overflow-x-auto" style={{ background: '#fff' }}>
        {tabs.map(tb => (
          <button key={tb.id} onClick={() => setTab(tb.id)}
            className={`px-4 py-3 text-sm border-b-2 -mb-px transition-colors whitespace-nowrap ${
              tab === tb.id ? 'border-[#1a73e8] text-[#1a73e8] font-medium' : 'border-transparent text-[#5f6368] hover:text-[#202124] hover:bg-[#f1f3f4]'}`}>
            {tb.label}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        <div className="max-w-3xl mx-auto px-8 py-6">
          {tab === 'preferences' && <PreferencesTab />}
          {tab === 'caldav'      && <CalDavTab />}
          {tab === 'about'       && <AboutTab />}
        </div>
      </div>
    </div>
  )
}

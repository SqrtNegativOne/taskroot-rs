export type SettingValue = string | number | boolean;

export type SettingType = 'select' | 'checkbox' | 'number' | 'time' | 'keybinding' | 'custom' | 'action';

export interface SettingOption {
    value: SettingValue;
    label: string;
}

export interface SettingsSchemaItem {
    id: string;
    label: string;
    description?: string;
    keywords?: string[];
    type: SettingType;
    options?: SettingOption[];
    min?: number;
    max?: number;
    defaultValue?: SettingValue;
    danger?: boolean;
}

export interface SettingSection {
    name: string;
    settings: SettingsSchemaItem[];
}

export interface SettingTab {
    id: string;
    label: string;
    sections: SettingSection[];
}

export interface SettingsSchema {
    tabs: SettingTab[];
}

import type { AppSettings as GeneratedAppSettings } from '../../lib/bindings/AppSettings.generated';

/**
 * Generated from src-tauri/settings.yaml by src-tauri/build.rs (ts-rs export).
 * Intersected with a string-key record so schema rows can index it by setting id.
 */
export type AppSettings = GeneratedAppSettings & Record<string, SettingValue>;

import type { TaskPriority } from './bindings/TaskPriority.generated';

export interface SigilProperties {
    priority?: TaskPriority;
    tags: string[];
    duration?: number;
    day?: string;
}

export interface ParsedSigils {
    cleanTitle: string;
    properties: SigilProperties;
}

function applySigilWord(word: string, properties: SigilProperties): boolean {
    if (word.startsWith('#') && word.length > 1) {
        properties.tags.push(word.slice(1));
        return true;
    }
    if (word.startsWith('!')) {
        const pri = parseInt(word.slice(1), 10);
        if (!isNaN(pri) && pri >= 0 && pri <= 4) {
            properties.priority = pri as TaskPriority;
            return true;
        }
    }
    if (word.startsWith('^') && word.length > 1) {
        const est = parseDuration(word.slice(1));
        if (est !== undefined) {
            properties.duration = est;
            return true;
        }
    }
    if (word.startsWith('@') && word.length > 1) {
        properties.day = word.slice(1);
        return true;
    }
    return false;
}

export function parseSigils(input: string): ParsedSigils {
    const titleWords: string[] = [];
    const properties: SigilProperties = {
        tags: [],
    };

    const words = input.split(/\s+/).filter(w => w.length > 0);

    for (const word of words) {
        if (!applySigilWord(word, properties)) {
            titleWords.push(word);
        }
    }

    return {
        cleanTitle: titleWords.join(' '),
        properties,
    };
}

function parseDuration(s: string): number | undefined {
    if (s.length === 0) return undefined;
    if (s.endsWith('m')) {
        const val = parseInt(s.slice(0, -1), 10);
        return isNaN(val) ? undefined : val;
    }
    if (s.endsWith('h')) {
        const val = parseInt(s.slice(0, -1), 10);
        return isNaN(val) ? undefined : Math.max(0, val * 60);
    }
    const val = parseInt(s, 10);
    return isNaN(val) ? undefined : val;
}

export const DEFAULT_BAR_WIDTH = 340;
export const DEFAULT_BAR_HEIGHT = 48;

const MENU_SEARCH_HEIGHT = 44;
const MENU_MIN_HEIGHT = 120;
const MENU_MAX_HEIGHT = 340;
const MENU_ROW_HEIGHT = 34;
const MENU_CHROME_HEIGHT = 8;

export interface WorkArea {
    readonly top: number;
    readonly bottom: number;
}

export interface MenuLayout {
    readonly placement: 'up' | 'down';
    readonly menuHeight: number;
    readonly windowHeight: number;
    readonly windowY: number;
}

function desiredMenuHeight(rowCount: number): number {
    const content = MENU_SEARCH_HEIGHT + rowCount * MENU_ROW_HEIGHT + MENU_CHROME_HEIGHT;
    return Math.min(MENU_MAX_HEIGHT, Math.max(MENU_MIN_HEIGHT, content));
}

export function computeMenuLayout(
    barY: number,
    rowCount: number,
    workArea: WorkArea | null,
    barHeight = DEFAULT_BAR_HEIGHT,
): MenuLayout {
    const desired = desiredMenuHeight(rowCount);
    const availableBelow = workArea === null ? desired : workArea.bottom - barY - barHeight;
    const availableAbove = workArea === null ? desired : barY - workArea.top;
    const placement = availableBelow < desired && availableAbove > availableBelow ? 'up' : 'down';
    const available = placement === 'down' ? availableBelow : availableAbove;
    const menuHeight = Math.max(MENU_MIN_HEIGHT, Math.min(desired, available));

    return {
        placement,
        menuHeight,
        windowHeight: barHeight + menuHeight,
        windowY: placement === 'up' ? barY - menuHeight : barY,
    };
}

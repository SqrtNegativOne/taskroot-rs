/** How many chips a cell can show before a summary chip replaces the last slot. */
export interface ChipOverflow {
    visible: number;
    hidden: number;
}

/**
 * Split `total` chips into visible chips and the number summarised by the
 * trailing "+x" chip. When they do not all fit, the last fitting slot is used
 * by the summary, so `visible` is one less than the number of slots.
 *
 * A `slots` value that is not finite means the cell has not been measured yet;
 * render everything in that case.
 */
export function chipOverflow(total: number, slots: number): ChipOverflow {
    const count = Math.max(0, Math.floor(total));
    if (count === 0) return { visible: 0, hidden: 0 };

    const fitting = Number.isFinite(slots) ? Math.max(0, Math.floor(slots)) : count;
    if (count <= fitting) return { visible: count, hidden: 0 };
    if (fitting === 0) return { visible: 0, hidden: 0 };

    const visible = fitting - 1;
    return { visible, hidden: count - visible };
}

import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import RecurrenceEditor from './RecurrenceEditor.svelte';

describe('RecurrenceEditor', () => {
    it('displays the human-readable rrule.toText() summary for custom rules', () => {
        render(RecurrenceEditor, {
            props: {
                value: 'RRULE:FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,FR',
                onchange: vi.fn(),
            },
        });

        const trigger = screen.getByRole('button', { name: 'Repeat' });
        expect(trigger.textContent).toContain('Every 2 weeks on Monday, Friday');
        expect(screen.getByRole('button', { name: /customize/i })).toBeTruthy();
    });

    it('builds a custom recurrence rule with interval and weekdays and previews it live', async () => {
        const onchange = vi.fn();
        render(RecurrenceEditor, {
            props: {
                value: undefined,
                onchange,
            },
        });

        await fireEvent.click(screen.getByRole('button', { name: 'Repeat' }));
        await fireEvent.click(screen.getByRole('button', { name: 'Custom' }));

        const intervalInput = screen.getByRole('spinbutton', { name: 'Repeat interval' });
        await fireEvent.input(intervalInput, { target: { value: '2' } });
        await fireEvent.click(screen.getByRole('button', { name: 'Monday' }));
        await fireEvent.click(screen.getByRole('button', { name: 'Wednesday' }));

        expect(screen.getByText('Every 2 weeks on Monday, Wednesday')).toBeTruthy();

        await fireEvent.click(screen.getByRole('button', { name: 'Apply' }));

        expect(onchange).toHaveBeenCalledExactlyOnceWith(
            'RRULE:FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE',
        );
    });

    it('supports custom count and until end conditions', async () => {
        const onchange = vi.fn();
        render(RecurrenceEditor, {
            props: {
                value: 'RRULE:FREQ=DAILY',
                onchange,
            },
        });

        await fireEvent.click(screen.getByRole('button', { name: 'Repeat' }));
        await fireEvent.click(screen.getByRole('button', { name: 'Custom' }));

        await fireEvent.click(screen.getByRole('button', { name: 'Recurrence end' }));
        await fireEvent.click(screen.getByRole('button', { name: 'After' }));
        await fireEvent.input(screen.getByRole('spinbutton', { name: 'Repeat count' }), {
            target: { value: '5' },
        });
        expect(screen.getByText('Every day for 5 times')).toBeTruthy();

        await fireEvent.click(screen.getByRole('button', { name: 'Apply' }));
        expect(onchange).toHaveBeenCalledWith('RRULE:FREQ=DAILY;COUNT=5');
    });

    it('toggles raw RRULE editing with live preview and validation feedback', async () => {
        const onchange = vi.fn();
        render(RecurrenceEditor, {
            props: {
                value: 'RRULE:FREQ=WEEKLY',
                onchange,
            },
        });

        await fireEvent.click(screen.getByRole('button', { name: /advanced/i }));

        const rawInput = screen.getByRole('textbox', { name: 'Raw recurrence rule' });
        await fireEvent.input(rawInput, { target: { value: 'RRULE:FREQ=BOGUS' } });
        expect(screen.getByRole('alert').textContent).toContain('valid RFC 5545');
        await fireEvent.change(rawInput, { target: { value: 'RRULE:FREQ=BOGUS' } });
        expect(onchange).not.toHaveBeenCalled();

        await fireEvent.input(rawInput, { target: { value: 'RRULE:FREQ=MONTHLY;BYMONTHDAY=15' } });
        expect(screen.queryByRole('alert')).toBeNull();
        expect(screen.getByText('Every month on the 15th')).toBeTruthy();

        await fireEvent.change(rawInput, { target: { value: 'RRULE:FREQ=MONTHLY;BYMONTHDAY=15' } });
        expect(onchange).toHaveBeenCalledExactlyOnceWith('RRULE:FREQ=MONTHLY;BYMONTHDAY=15');
    });
});

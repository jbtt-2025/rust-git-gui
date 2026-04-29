import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, fireEvent, cleanup } from '@testing-library/react';
import { RepoSearch } from '../RepoSearch';

// Mock react-i18next
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const translations: Record<string, string> = {
        'welcome.searchPlaceholder': 'Search repositories...',
      };
      return translations[key] ?? key;
    },
  }),
}));

afterEach(() => {
  cleanup();
});

describe('RepoSearch', () => {
  it('renders a search input with the i18n placeholder', () => {
    render(<RepoSearch value="" onChange={vi.fn()} onClear={vi.fn()} />);
    const input = screen.getByPlaceholderText('Search repositories...');
    expect(input).toBeDefined();
  });

  it('displays the controlled value', () => {
    render(<RepoSearch value="my-repo" onChange={vi.fn()} onClear={vi.fn()} />);
    const input = screen.getByRole('searchbox') as HTMLInputElement;
    expect(input.value).toBe('my-repo');
  });

  it('calls onChange when the user types', () => {
    const onChange = vi.fn();
    render(<RepoSearch value="" onChange={onChange} onClear={vi.fn()} />);
    const input = screen.getByRole('searchbox');
    fireEvent.change(input, { target: { value: 'test' } });
    expect(onChange).toHaveBeenCalledWith('test');
  });

  it('calls onClear when Escape is pressed', () => {
    const onClear = vi.fn();
    render(<RepoSearch value="query" onChange={vi.fn()} onClear={onClear} />);
    const input = screen.getByRole('searchbox');
    fireEvent.keyDown(input, { key: 'Escape' });
    expect(onClear).toHaveBeenCalledOnce();
  });

  it('does not call onClear for non-Escape keys', () => {
    const onClear = vi.fn();
    render(<RepoSearch value="" onChange={vi.fn()} onClear={onClear} />);
    const input = screen.getByRole('searchbox');
    fireEvent.keyDown(input, { key: 'Enter' });
    fireEvent.keyDown(input, { key: 'a' });
    expect(onClear).not.toHaveBeenCalled();
  });

  it('has an aria-label for accessibility', () => {
    render(<RepoSearch value="" onChange={vi.fn()} onClear={vi.fn()} />);
    const input = screen.getByLabelText('Search repositories...');
    expect(input).toBeDefined();
  });

  it('has focus ring styling via className', () => {
    render(<RepoSearch value="" onChange={vi.fn()} onClear={vi.fn()} />);
    const input = screen.getByRole('searchbox');
    expect(input.className).toContain('focus:ring-2');
    expect(input.className).toContain('focus:ring-blue-500');
  });
});

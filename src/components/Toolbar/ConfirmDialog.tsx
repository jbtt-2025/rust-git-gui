import * as Dialog from '@radix-ui/react-dialog';
import { useTranslation } from 'react-i18next';

export interface ConfirmDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description: string;
  confirmLabel?: string;
  cancelLabel?: string;
  onConfirm: () => void;
  variant?: 'danger' | 'default';
}

export function ConfirmDialog({
  open,
  onOpenChange,
  title,
  description,
  confirmLabel,
  cancelLabel,
  onConfirm,
  variant = 'default',
}: ConfirmDialogProps) {
  const { t } = useTranslation();

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="confirm-dialog-overlay" />
        <Dialog.Content className="confirm-dialog-content" aria-describedby="confirm-desc">
          <Dialog.Title className="confirm-dialog-title">{title}</Dialog.Title>
          <Dialog.Description id="confirm-desc" className="confirm-dialog-description">
            {description}
          </Dialog.Description>
          <div className="confirm-dialog-actions">
            <Dialog.Close asChild>
              <button className="confirm-dialog-btn confirm-dialog-btn--cancel" type="button">
                {cancelLabel ?? t('common.cancel')}
              </button>
            </Dialog.Close>
            <button
              className={`confirm-dialog-btn confirm-dialog-btn--confirm${variant === 'danger' ? ' confirm-dialog-btn--danger' : ''}`}
              type="button"
              onClick={() => {
                onConfirm();
                onOpenChange(false);
              }}
            >
              {confirmLabel ?? t('common.confirm')}
            </button>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

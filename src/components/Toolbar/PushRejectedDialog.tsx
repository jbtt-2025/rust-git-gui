import * as Dialog from '@radix-ui/react-dialog';
import { useTranslation } from 'react-i18next';

export interface PushRejectedDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onPullThenPush: () => void;
  onForcePush: () => void;
}

export function PushRejectedDialog({
  open,
  onOpenChange,
  onPullThenPush,
  onForcePush,
}: PushRejectedDialogProps) {
  const { t } = useTranslation();

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="confirm-dialog-overlay" />
        <Dialog.Content className="confirm-dialog-content" aria-describedby="push-rejected-desc">
          <Dialog.Title className="confirm-dialog-title">
            {t('toolbar.pushRejected')}
          </Dialog.Title>
          <Dialog.Description id="push-rejected-desc" className="confirm-dialog-description">
            {t('toolbar.pushRejectedDesc')}
          </Dialog.Description>
          <div className="confirm-dialog-actions">
            <Dialog.Close asChild>
              <button className="confirm-dialog-btn confirm-dialog-btn--cancel" type="button">
                {t('common.cancel')}
              </button>
            </Dialog.Close>
            <button
              className="confirm-dialog-btn confirm-dialog-btn--confirm"
              type="button"
              onClick={() => {
                onPullThenPush();
                onOpenChange(false);
              }}
            >
              {t('toolbar.pullThenPush')}
            </button>
            <button
              className="confirm-dialog-btn confirm-dialog-btn--confirm confirm-dialog-btn--danger"
              type="button"
              onClick={() => {
                onForcePush();
                onOpenChange(false);
              }}
            >
              {t('toolbar.forcePush')}
            </button>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

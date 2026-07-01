interface Props {
  phase: string;
  seconds: number;
}

export function CookingIndicator({ phase, seconds }: Props) {
  return (
    <div className="cooking-message">
      <span className="cooking-message__text">{phase || '✨ Working…'}</span>
      <span className="cooking-message__timer">{seconds}s</span>
    </div>
  );
}

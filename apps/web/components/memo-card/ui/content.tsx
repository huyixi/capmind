"use client";

import {
  useCallback,
  useId,
  useEffect,
  useRef,
  useState,
  type ChangeEvent,
  type KeyboardEvent,
  type ReactNode,
  type RefObject,
} from "react";
import { createPortal } from "react-dom";
import Image from "next/image";
import { ImageIcon, Loader2 } from "lucide-react";
import {
  SEE_LESS_LABEL,
  SEE_MORE_LABEL,
} from "../logic/constants";
import { isLocalImageUrl } from "../logic/image-utils";

interface MemoImageThumbProps {
  imageUrl: string;
  index: number;
  onClick: (index: number) => void;
  onHover: () => void;
  useNextImage: boolean;
}

function MemoImageThumb({
  imageUrl,
  index,
  onClick,
  onHover,
  useNextImage,
}: MemoImageThumbProps) {
  const src = imageUrl || "/placeholder.svg";

  return (
    <button
      type="button"
      onClick={() => onClick(index)}
      onMouseEnter={onHover}
      onFocus={onHover}
      className="relative shrink-0 overflow-hidden rounded-lg border border-border transition-colors hover:border-primary/50"
    >
      {useNextImage ? (
        <Image
          src={src}
          alt={`Memo image ${index + 1}`}
          width={120}
          height={120}
          sizes="120px"
          className="object-cover w-30 h-30"
        />
      ) : (
        <>
          {/* eslint-disable-next-line @next/next/no-img-element */}
          <img
            src={src}
            alt={`Memo image ${index + 1}`}
            width={120}
            height={120}
            className="object-cover w-30 h-30"
          />
        </>
      )}
      <div className="absolute inset-0 bg-background/0 hover:bg-background/10 transition-colors flex items-center justify-center">
        <ImageIcon className="h-6 w-6 text-foreground opacity-0 hover:opacity-100 transition-opacity" />
      </div>
    </button>
  );
}

interface MemoCardContentProps {
  memoId: string;
  memoText: string;
  linkifiedMemoText: ReactNode;
  contentId: string;
  contentRef: RefObject<HTMLParagraphElement | null>;
  shouldShowToggle: boolean;
  displayImages: string[];
  memoImageCount: number;
  needsImageFetch: boolean;
  isResolvingImages: boolean;
  onResolvePendingImages: () => void;
  onImageClick: (index: number) => void;
  onImageHover: () => void;
  imageAreaRef: RefObject<HTMLDivElement | null>;
}

export function MemoCardContent({
  memoId,
  memoText,
  linkifiedMemoText,
  contentId,
  contentRef,
  shouldShowToggle,
  displayImages,
  memoImageCount,
  needsImageFetch,
  isResolvingImages,
  onResolvePendingImages,
  onImageClick,
  onImageHover,
  imageAreaRef,
}: MemoCardContentProps) {
  const shouldShowPlaceholders =
    displayImages.length === 0 &&
    memoImageCount > 0 &&
    (needsImageFetch || isResolvingImages);
  const shouldShowImages = displayImages.length > 0 || shouldShowPlaceholders;
  const expandToggleId = useId();
  const cardContentRef = useRef<HTMLDivElement | null>(null);
  const expandInputRef = useRef<HTMLInputElement | null>(null);
  const toggleRef = useRef<HTMLLabelElement | null>(null);
  const [isExpanded, setIsExpanded] = useState(false);
  const [isInlineToggleVisible, setIsInlineToggleVisible] = useState(true);
  const [isCardInViewport, setIsCardInViewport] = useState(true);
  const shouldShowStickyCollapse =
    shouldShowToggle && isExpanded && !isInlineToggleVisible && isCardInViewport;

  useEffect(() => {
    if (!shouldShowToggle) {
      setIsExpanded(false);
      setIsInlineToggleVisible(true);
      return;
    }

    const inputElement = expandInputRef.current;
    if (!inputElement) {
      return;
    }

    setIsExpanded(inputElement.checked);
  }, [shouldShowToggle]);

  useEffect(() => {
    if (!shouldShowToggle || !isExpanded) {
      setIsCardInViewport(true);
      return;
    }
    const cardElement = cardContentRef.current;
    if (!cardElement) return;

    const updateViewportState = () => {
      const rect = cardElement.getBoundingClientRect();
      setIsCardInViewport(rect.bottom > 0 && rect.top < window.innerHeight);
    };

    updateViewportState();
    if (typeof IntersectionObserver === "undefined") return;

    const observer = new IntersectionObserver(
      ([entry]) => {
        setIsCardInViewport(entry.isIntersecting);
      },
      {
        root: null,
        threshold: 0,
      },
    );
    observer.observe(cardElement);
    return () => observer.disconnect();
  }, [isExpanded, shouldShowToggle]);

  useEffect(() => {
    if (!shouldShowToggle) return;
    const toggleElement = toggleRef.current;
    if (!toggleElement) return;
    if (typeof IntersectionObserver === "undefined") return;

    const observer = new IntersectionObserver(
      ([entry]) => {
        setIsInlineToggleVisible(entry.isIntersecting);
      },
      {
        root: null,
        threshold: 0,
      },
    );
    observer.observe(toggleElement);
    return () => observer.disconnect();
  }, [shouldShowToggle]);

  const handleExpandChange = useCallback((event: ChangeEvent<HTMLInputElement>) => {
    setIsExpanded(event.currentTarget.checked);
  }, []);

  const handleToggleKeyDown = useCallback(
    (event: KeyboardEvent<HTMLLabelElement>) => {
      if (event.key !== "Enter" && event.key !== " ") return;
      event.preventDefault();
      const input = expandInputRef.current;
      if (!input) return;
      const nextChecked = !input.checked;
      input.checked = nextChecked;
      setIsExpanded(nextChecked);
    },
    [],
  );

  const handleStickyCollapse = useCallback(() => {
    if (!expandInputRef.current) return;
    expandInputRef.current.checked = false;
    setIsExpanded(false);
  }, []);

  const stickyCollapseButton = shouldShowStickyCollapse ? (
    <button
      type="button"
      className="memo-card-sticky-toggle"
      onClick={handleStickyCollapse}
      aria-controls={contentId}
    >
      {SEE_LESS_LABEL}
    </button>
  ) : null;

  if (memoText.length === 0 && !shouldShowImages) return null;

  return (
    <div ref={cardContentRef} className="min-w-0">
      {memoText.length > 0 ? (
        shouldShowToggle ? (
          <div className="memo-card-collapse">
            <input
              id={expandToggleId}
              ref={expandInputRef}
              type="checkbox"
              className="memo-card-expand-input"
              onChange={handleExpandChange}
            />
            <p
              id={contentId}
              ref={contentRef}
              className="memo-card-text text-foreground whitespace-pre-wrap wrap-break-words"
            >
              {linkifiedMemoText}
            </p>
            <label
              ref={toggleRef}
              className="memo-card-toggle"
              htmlFor={expandToggleId}
              role="button"
              tabIndex={0}
              aria-expanded={isExpanded}
              aria-controls={contentId}
              onKeyDown={handleToggleKeyDown}
            >
              <span className="memo-card-toggle-more">{SEE_MORE_LABEL}</span>
              <span className="memo-card-toggle-less">{SEE_LESS_LABEL}</span>
            </label>
          </div>
        ) : (
          <p
            id={contentId}
            ref={contentRef}
            className="memo-card-text text-foreground whitespace-pre-wrap wrap-break-words"
          >
            {linkifiedMemoText}
          </p>
        )
      ) : null}
      {typeof document !== "undefined" && stickyCollapseButton
        ? createPortal(stickyCollapseButton, document.body)
        : null}

      {shouldShowImages && (
        <div
          ref={imageAreaRef}
          className="mt-3 flex flex-nowrap gap-2 overflow-x-auto"
        >
          {shouldShowPlaceholders
            ? Array.from({ length: memoImageCount }).map((_, index) => (
                <button
                  key={`memo-placeholder-${memoId}-${index}`}
                  type="button"
                  onClick={onResolvePendingImages}
                  className="relative flex h-30 w-30 shrink-0 items-center justify-center rounded-lg border border-border bg-secondary/40 text-muted-foreground transition-colors hover:border-primary/50"
                  aria-label="Load memo images"
                >
                  {isResolvingImages ? (
                    <Loader2 className="h-5 w-5 animate-spin" />
                  ) : (
                    <ImageIcon className="h-6 w-6" />
                  )}
                </button>
              ))
            : displayImages.map((imageUrl, index) =>
                <MemoImageThumb
                  key={imageUrl}
                  imageUrl={imageUrl}
                  index={index}
                  onClick={onImageClick}
                  onHover={onImageHover}
                  useNextImage={!isLocalImageUrl(imageUrl)}
                />,
              )}
        </div>
      )}
    </div>
  );
}

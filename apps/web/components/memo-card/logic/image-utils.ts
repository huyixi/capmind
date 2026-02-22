"use client";

export const isLocalImageUrl = (url: string) =>
  url.startsWith("blob:") || url.startsWith("data:");

export const isRemoteImageUrl = (url: string) =>
  url.startsWith("http://") || url.startsWith("https://");

export const areImagesEqual = (left: string[], right: string[]) =>
  left.length === right.length &&
  left.every((value, index) => value === right[index]);

import type { ListIntakeReviewReport } from "./domain/listIntakeReview"

export const visualListIntakeReport = {
  generatedAtUnixSeconds: 1_783_000_000,
  aggregates: [
    {
      profileId: "list-intake-fishcount",
      profileName: "Fish count",
      outputPolicy: "Aggregate only",
      localDate: "2026-07-07",
      chatLabel: "Chat alpha",
      senderLabel: "Sender 1",
      categoryLabel: "Seafood",
      items: [
        {
          itemName: "anchovies",
          quantity: 5,
          unit: "count",
          categoryId: "seafood",
          categoryLabel: "Seafood"
        },
        {
          itemName: "salmon",
          quantity: 3,
          unit: "count",
          categoryId: "seafood",
          categoryLabel: "Seafood"
        }
      ]
    }
  ],
  proposals: [
    {
      proposalId: "list-intake-proposal-fixture",
      profileName: "List intake proposal",
      chatLabel: "Chat alpha",
      senderLabel: "Sender 1",
      items: [
        {
          itemName: "anchovies",
          quantity: 5,
          unit: "count",
          categoryId: "seafood",
          categoryLabel: "Seafood"
        }
      ],
      categories: [
        { categoryId: "seafood", label: "Seafood" },
        { categoryId: "uncategorized", label: "Uncategorized" }
      ]
    }
  ]
} as const satisfies ListIntakeReviewReport

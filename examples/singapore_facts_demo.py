#!/usr/bin/env python3
"""Singapore Facts with Mnemo Memory Example

This example demonstrates how to use the `mnemo` memory library to store and
recall facts about Singapore, comparing the manual (agentic) approach with
the automated extraction approach.
"""

import json
import sys
from typing import List, Optional


def demonstrate_manual_approach():
    """
    Demonstrates the manual approach where the developer explicitly calls
    `mnemo_remember` for each fact they want to store.
    
    Benefits:
    - Full control over what is stored and its importance.
    - Suitable for critical facts where precision is required.
    - Can be easily extended to prompt a user for confirmation.
    
    Drawbacks:
    - Tedious and requires more code for large texts.
    - Requires the developer to identify and summarize key facts manually.
    """
    print("=" * 60)
    print("MANUAL APPROACH: Explicitly storing key facts")
    print("=" * 60)

    facts = {
        "geography": "Singapore is an island country & city-state in Southeast Asia, located north of the equator.",
        "name_origin": 'The name "Singapore" comes from Malay Singapura, derived from Sanskrit Simhapura, meaning "Lion City".',
        "languages": "Official languages of Singapore: English, Malay, Mandarin, Tamil. Malay is the national language.",
        "ethnicity": "Singapore's ethnic composition (2023): Chinese 74.3%, Malay 13.5%, Indian 9.0%, Other 3.2%.",
        "history": "Singapore gained independence from Malaysia on 9 August 1965. The first PM was Lee Kuan Yew."
    }

    # In a real application, the developer would import and use the library.
    # We simulate the calls here to show what the logic would look like.
    # from mnemo import remember, recall
    print("\n[Simulation] Calling mnemo.store() for each fact...")
    for category, fact in facts.items():
        # Example: remember(fact, memory_type="semantic", importance=0.9)
        print(f"  - Storing ({category}): {fact}")
    print("\n✓ All facts manually stored with high importance (0.9).")


def demonstrate_automated_approach():
    """
    Demonstrates the automated approach using `mnemo` extraction or binding
    functionality where the library automatically identifies and stores facts.
    
    Benefits:
    - Fast and requires minimal code.
    - Good for bulk ingestion of unstructured text.
    - Can discover facts a human might miss.
    
    Drawbacks:
    - Less control over what is stored.
    - May store irrelevant or low-importance details.
    - Importance levels are determined by the system, not the user.
    """
    print("\n" + "=" * 60)
    print("AUTOMATED APPROACH: Using mnemo.extract()")
    print("=" * 60)

    wikipedia_text = """
    Singapore, officially the Republic of Singapore, is an island country and city-state in Southeast Asia. 
    Its territory comprises a main island, over 60 satellite islands and islets, and one outlying islet. 
    The country is about one degree of latitude (137 kilometres or 85 miles) north of the equator, off the 
    southern tip of the Malay Peninsula, bordering the Strait of Malacca to the west, the Singapore Strait 
    to the south along with the Riau Islands in Indonesia, the South China Sea to the east and the Straits 
    of Johor along with the State of Johor in Malaysia to the north.

    The English name of "Singapore" is an anglicisation of the native Malay name for the country, 
    Singapura, which was in turn derived from the Sanskrit word for 'lion city'.
    
    Singapore's population is approximately 6.1 million. The ethnic composition is 74.3% Chinese, 
    13.5% Malay, 9.0% Indian, and 3.2% Other. The official languages are English, Malay, Mandarin, 
    and Tamil, with Malay as the national language.
    
    Singapore became independent on 9 August 1965. The People's Action Party (PAP) has been in power 
    continuously since 1959. The current Prime Minister is Lawrence Wong.
    """

    print("\n[Simulation] Calling mnemo.extract(text, auto_store=True)...")
    print(f"  - Input text length: {len(wikipedia_text)} characters")
    
    # In a real application:
    # extracted_memories = mnemo.extract(wikipedia_text, auto_store=True)
    # print(f"  - Extracted {len(extracted_memories)} memories automatically.")
    
    print("  - Extracted 4 memories automatically based on heuristics.")
    print("✓ Facts automatically identified and stored by the system.")


def demonstrate_recall():
    """
    Demonstrates how to recall stored facts using the manual vs automated 
    approach, showing that the end result is functionally the same for the user.
    """
    print("\n" + "=" * 60)
    print("RECALL DEMO: Retrieving stored facts")
    print("=" * 60)

    queries = [
        "What is the origin of Singapore's name?",
        "What is the population of Singapore?",
        "When did Singapore become independent?",
        "What is the ethnic composition of Singapore?"
    ]

    print("\n[Simulation] Calling mnemo.recall(query, limit=3) for each question...")
    for query in queries:
        print(f"\n  > User: {query}")
        # In a real application, results = mnemo.recall(query, limit=3)
        # We simulate a relevant result
        print(f"  < Agent: [Relevant fact retrieved from memory storage]")


def demonstrate_mixed_approach():
    """
    Shows the recommended hybrid approach: Use automated extraction for bulk text,
    then use manual storage for critical facts that were missed or need higher importance.
    """
    print("\n" + "=" * 60)
    print("HYBRID APPROACH: Combining manual and automated storage")
    print("=" * 60)

    wikipedia_text = "Singapore's GDP per capita is one of the highest in the world."
    
    print("\nStep 1: Automated extraction for initial processing...")
    # mnemo.extract(wikipedia_text, auto_store=True)
    print("  - Basic GDP fact automatically stored.")
    
    print("\nStep 2: Manual override for critical context...")
    # mnemo.remember("Singapore has the highest PPP-adjusted GDP per capita in the world. ", importance=0.95)
    # mnemo.remember("Singapore is the only Asian country with AAA credit rating from all major agencies.", importance=0.95)
    print("  - High-importance economic facts manually stored with importance=0.95.")
    print("  - This ensures key stats are prioritized in future recalls.")

    print("\n✓ Hybrid approach leverages automation while ensuring critical data is accurately captured.")


def main():
    """Main execution function orchestrating the demo."""
    print("Mnemo Memory Library: Singapore Facts Demo")
    print("-" * 60)
    
    demonstrate_manual_approach()
    demonstrate_automated_approach()
    demonstrate_recall()
    demonstrate_mixed_approach()

    print("\n" + "=" * 60)
    print("SUMMARY")
    print("=" * 60)
    print("""
Both manual and automated approaches serve the same ultimate goal: 
persistence and recollection of information. 

The manual approach requires a deliberate decision to store facts, 
while the automated approach streamlines the process by guessing 
which snippets are worth keeping.

The choice between them boils down to a trade-off between control 
and convenience, but the underlying mechanism (storing text, 
retrieving on query) remains exactly the same.
""")


if __name__ == "__main__":
    main()

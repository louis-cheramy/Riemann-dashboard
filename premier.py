import math
import struct
import os

def simple_sieve(limit):
    est_premier = [True] * (limit + 1)
    est_premier[0:2] = [False, False]

    for i in range(2, int(math.sqrt(limit)) + 1):
        if est_premier[i]:
            for j in range(i * i, limit + 1, i):
                est_premier[j] = False

    return [i for i, val in enumerate(est_premier) if val]

def crible_segmenté(n, segment_size=10_000_000):
    print(f"Recherche de tous les nombres premiers jusqu'à {n}...")
    limite_racine = int(math.sqrt(n)) + 1
    premiers_base = simple_sieve(limite_racine)
    print("Chemin complet du fichier :", os.path.abspath("nombres_premiers.bin"))
    
    with open("nombres_premiers.bin", "wb") as fichier:
        for p in premiers_base:
            fichier.write(struct.pack("<I", p))  # "<I" = unsigned 32-bit little-endian

        debut = limite_racine
        while debut < n:
            fin = min(debut + segment_size, n + 1)
            est_premier = [True] * (fin - debut)

            for p in premiers_base:
                debut_mult = max(p * p, ((debut + p - 1) // p) * p)
                for j in range(debut_mult, fin, p):
                    est_premier[j - debut] = False

            for i in range(debut, fin):
                if est_premier[i - debut]:
                    fichier.write(struct.pack("<I", i))

            print(f"Segment {debut:,} → {fin - 1:,} traité.")
            debut = fin

    print("Tous les nombres premiers ont été enregistrés dans 'nombres_premiers.bin' (format compact).")

if __name__ == "__main__":
    import time
    try:
        n = int(input("Entrez la borne maximale (ex: 10000000000) : "))
        start = time.time()
        crible_segmenté(n)
        duration = time.time() - start
        print(f"Terminé en {duration / 60:.2f} minutes.")
    except ValueError:
        print("Erreur : entrez un nombre entier valide.")

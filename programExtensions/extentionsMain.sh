extentions_location="$bashC/programExtensions";

load_extentions () {
    local program_extentions=(
        "dotnet" \
        "git" \
        "kubernetes" \
        "nvim" \
        "pnpm" \
        "rust" \
        "terraform" \
        "yarn" \
    )

    for i in "${program_extentions[@]}"
	do
		source "$extentions_location/$i/${i}Main.sh";
        alias my${i}show="$extentions_location/$i/${i}Show.sh";
        alias my${1}code="code $extentions_location/$i";
	done
}

load_extentions